mod error;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand};
use policy::Policy;
use runtime::{AnthropicBackend, Session};
use storage::{Event, EventKind, EventStore, Role};

use error::{Error, Result};

const SYSTEM_PROMPT: &str = "You are Bosun, a helpful AI assistant. Be concise and direct.";
const POLICY_FILE: &str = "bosun.toml";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

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

    // Get API key from environment
    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| Error::MissingApiKey)?;

    // Get model from environment or use default
    let model = std::env::var("BOSUN_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    // Initialize LLM backend
    let backend = AnthropicBackend::builder(api_key, &model).build();

    // Initialize event store
    let data_dir = dirs_data_dir().unwrap_or_else(|| ".bosun".into());
    std::fs::create_dir_all(&data_dir)?;
    let db_path = data_dir.join("events.db");
    let store = EventStore::open(&db_path)?;

    println!("Session stored at: {}", db_path.display());

    // Load policy
    let policy = load_policy()?;
    println!(
        "Policy: {}",
        if std::path::Path::new(POLICY_FILE).exists() {
            POLICY_FILE
        } else {
            "default (restrictive)"
        }
    );

    // Create session
    let mut session = Session::new(store, backend, policy)?.with_system(SYSTEM_PROMPT);
    println!("Session ID: {}", session.id);
    println!("Model: {model}");
    println!("Type 'quit' or Ctrl+D to exit.\n");

    // Chat loop
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            // EOF
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "quit" || input == "exit" {
            break;
        }

        match session.chat(input).await {
            Ok(response) => {
                println!("\n{response}\n");
            }
            Err(e) => {
                eprintln!("Error: {e}\n");
            }
        }
    }

    session.end()?;
    println!("\nSession ended.");
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
    println!("{}", "-".repeat(80));

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

    println!("Session: {session_id}\n");

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
            println!("[{time}] === Session started ===");
        }
        EventKind::SessionEnd => {
            println!("[{time}] === Session ended ===");
        }
        EventKind::Message { role, content } => {
            let role_str = match role {
                Role::User => "USER",
                Role::Assistant => "ASSISTANT",
                Role::System => "SYSTEM",
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
            println!("[{time}] TOOL CALL: {name} {input:?}");
        }
        EventKind::ToolResult { name, output } => {
            println!("[{time}] TOOL RESULT: {name} {output:?}");
        }
    }
}

fn open_store() -> Result<EventStore> {
    let data_dir = dirs_data_dir().unwrap_or_else(|| ".bosun".into());
    let db_path = data_dir.join("events.db");

    if !db_path.exists() {
        return Err(Error::DatabaseNotFound { path: db_path });
    }

    Ok(EventStore::open(&db_path)?)
}

fn dirs_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/bosun"))
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .map(|p| p.join("bosun"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|h| PathBuf::from(h).join("bosun"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

fn load_policy() -> Result<Policy> {
    let policy_path = PathBuf::from(POLICY_FILE);

    if policy_path.exists() {
        Ok(Policy::load(&policy_path)?)
    } else {
        Ok(Policy::restrictive())
    }
}
