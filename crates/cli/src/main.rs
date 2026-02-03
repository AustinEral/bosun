use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use policy::Policy;
use runtime::{Session, llm::Client};
use storage::EventStore;

const SYSTEM_PROMPT: &str = "You are Bosun, a helpful AI assistant. Be concise and direct.";
const POLICY_FILE: &str = "bosun.toml";

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("bosun v{}", env!("CARGO_PKG_VERSION"));

    // Initialize LLM client
    let client = Client::from_env()
        .map_err(|e| format!("{e}\nSet ANTHROPIC_API_KEY environment variable to use bosun."))?;

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
    let mut session = Session::new(store, client, policy)?.with_system(SYSTEM_PROMPT);
    println!("Session ID: {}", session.id);
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

fn dirs_data_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share/bosun"))
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"))
            })
            .map(|p| p.join("bosun"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|h| std::path::PathBuf::from(h).join("bosun"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

fn load_policy() -> Result<Policy, Box<dyn std::error::Error>> {
    let policy_path = PathBuf::from(POLICY_FILE);

    if policy_path.exists() {
        Ok(Policy::load(&policy_path)?)
    } else {
        // Use default restrictive policy
        Ok(Policy::restrictive())
    }
}
