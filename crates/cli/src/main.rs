mod config;
mod error;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand};
use runtime::{AnthropicBackend, EmptyToolHost, McpToolHost, Session, ToolHost};
use storage::{Event, EventKind, EventStore, Role};

use config::Config;
use error::{Error, Result};

const SYSTEM_PROMPT: &str = "You are Bosun, a helpful AI assistant. Be concise and direct.";
const CONFIG_FILE: &str = "bosun.toml";
const APP_NAME: &str = "bosun";

#[derive(Parser)]
#[command(name = "bosun")]
#[command(about = "A local-first AI agent runtime", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive chat session
    Chat,
    /// List all sessions
    Sessions {
        /// Show only the last N sessions
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Show event logs for a session
    Logs {
        /// Session ID (prefix match supported)
        #[arg(short, long)]
        session: String,
        /// Filter by event kind (message, tool_call, tool_result)
        #[arg(short, long)]
        kind: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Chat) | None => cmd_chat().await,
        Some(Commands::Sessions { limit }) => cmd_sessions(limit),
        Some(Commands::Logs { session, kind }) => cmd_logs(&session, kind.as_deref()),
    }
}

async fn cmd_chat() -> Result<()> {
    println!("bosun v{}", env!("CARGO_PKG_VERSION"));
    println!();

    // Load configuration
    let config = load_config()?;

    // Get authentication (from config or env)
    let auth = config.auth()?;

    // Initialize LLM backend
    let backend = AnthropicBackend::builder(auth, &config.backend.model)
        .system(SYSTEM_PROMPT)
        .build();

    // Initialize event store
    let data_dir = data_dir();
    std::fs::create_dir_all(&data_dir)?;
    let db_path = data_dir.join("events.db");
    let store = EventStore::open(&db_path)?;

    // Create session
    let mut session = Session::new(store, backend, config.policy)?;

    println!("  Model:   {}", config.backend.model);
    println!("  Session: {}", session.id);

    // Initialize tool host
    if let Some(tool_config) = config.tools.first() {
        let tool_host = McpToolHost::spawn(&tool_config.command, &tool_config.args)
            .await
            .map_err(|e| Error::Tool(e.to_string()))?;

        let tool_count = tool_host.specs().len();
        println!("  Tools:   {} from {}", tool_count, tool_config.command);
        println!();
        println!("Type 'quit' to exit.");
        println!("─────────────────────────────────────────");
        println!();

        chat_loop(&mut session, &tool_host).await
    } else {
        println!("  Tools:   none");
        println!();
        println!("Type 'quit' to exit.");
        println!("─────────────────────────────────────────");
        println!();

        chat_loop(&mut session, &EmptyToolHost).await
    }
}

async fn chat_loop<B, H>(session: &mut Session<B>, tool_host: &H) -> Result<()>
where
    B: runtime::Backend,
    H: ToolHost,
{
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("› ");
        stdout.flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "quit" || input == "exit" {
            break;
        }

        match session.chat_with_tools(input, tool_host).await {
            Ok((response, usage)) => {
                println!();
                println!("{response}");
                println!();
                println!("  {} in → {} out", usage.input_tokens, usage.output_tokens);
                println!();
            }
            Err(e) => {
                eprintln!("Error: {e}");
                println!();
            }
        }
    }

    // Session summary
    let total = session.usage();
    println!();
    println!("─────────────────────────────────────────");
    println!("  Session complete");
    println!(
        "  Tokens: {} in → {} out",
        total.input_tokens, total.output_tokens
    );
    println!("─────────────────────────────────────────");

    Ok(())
}

fn cmd_sessions(limit: usize) -> Result<()> {
    let store = open_store()?;
    let sessions = store.list_sessions()?;

    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    println!(
        "{:<36}  {:<20}  {:<8}  STATUS",
        "SESSION ID", "STARTED", "MSGS"
    );
    println!("{}", "─".repeat(80));

    for summary in sessions.into_iter().take(limit) {
        let started = Local
            .from_utc_datetime(&summary.started_at.naive_utc())
            .format("%Y-%m-%d %H:%M");
        let status = if summary.ended_at.is_some() {
            "ended"
        } else {
            "active"
        };
        println!(
            "{:<36}  {:<20}  {:<8}  {status}",
            summary.id, started, summary.message_count
        );
    }

    Ok(())
}

fn cmd_logs(session_prefix: &str, kind_filter: Option<&str>) -> Result<()> {
    let store = open_store()?;

    // Find session by prefix
    let sessions = store.list_sessions()?;
    let matching: Vec<_> = sessions
        .iter()
        .filter(|s| s.id.to_string().starts_with(session_prefix))
        .collect();

    let session_id = match matching.len() {
        0 => {
            return Err(Error::SessionNotFound {
                prefix: session_prefix.to_string(),
            });
        }
        1 => matching[0].id,
        _ => {
            return Err(Error::AmbiguousSession {
                prefix: session_prefix.to_string(),
                matches: matching.iter().map(|s| s.id.to_string()).collect(),
            });
        }
    };

    let events = store.load_events(session_id, kind_filter)?;

    if events.is_empty() {
        println!("No events found for session {session_id}");
        return Ok(());
    }

    println!("Session: {session_id}");
    println!();

    for event in events {
        print_event(&event);
    }

    Ok(())
}

fn print_event(event: &Event) {
    let time = Local
        .from_utc_datetime(&event.timestamp.naive_utc())
        .format("%H:%M:%S");

    match &event.kind {
        EventKind::SessionStart => {
            println!("[{time}] ─── Session started ───");
        }
        EventKind::SessionEnd => {
            println!("[{time}] ─── Session ended ───");
        }
        EventKind::Message { role, content } => {
            let role_str = match role {
                Role::User => "USER",
                Role::Assistant => "ASST",
                Role::System => "SYS",
            };
            // Truncate long messages for display
            let display_content = if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.clone()
            };
            println!("[{time}] {role_str}: {display_content}");
        }
        EventKind::ToolCall { name, input } => {
            println!("[{time}] CALL: {name} {input:?}");
        }
        EventKind::ToolResult { name, output } => {
            println!("[{time}] RESULT: {name} {output:?}");
        }
    }
}

fn load_config() -> Result<Config> {
    let config_path = PathBuf::from(CONFIG_FILE);

    if config_path.exists() {
        Ok(Config::load(&config_path)?)
    } else {
        Ok(Config::default_config())
    }
}

fn open_store() -> Result<EventStore> {
    let data_dir = data_dir();
    let db_path = data_dir.join("events.db");

    if !db_path.exists() {
        return Err(Error::DatabaseNotFound { path: db_path });
    }

    Ok(EventStore::open(&db_path)?)
}

/// Returns the platform-appropriate data directory for Bosun.
fn data_dir() -> PathBuf {
    dirs::data_dir()
        .map(|p| p.join(APP_NAME))
        .unwrap_or_else(|| PathBuf::from(".bosun"))
}
