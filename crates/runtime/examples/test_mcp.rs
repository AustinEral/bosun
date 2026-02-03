//! Quick test of MCP integration.
//!
//! Run with: cargo run --example test_mcp

use std::collections::HashMap;
use std::sync::Arc;

use mcp::ServerConfig;
use runtime::ToolHost;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing MCP integration...\n");

    // Configure filesystem MCP server
    let config = ServerConfig {
        name: "filesystem".to_string(),
        command: "mcp-server-filesystem".to_string(),
        args: vec!["/tmp".to_string()],
        env: HashMap::new(),
    };

    println!("Spawning MCP server: {}", config.name);
    println!("  Command: {} {:?}", config.command, config.args);

    // Create tool host and initialize
    let host = ToolHost::new(vec![config]);
    host.initialize().await?;

    // List tools
    let tools = host.list_tools().await;
    println!("\nDiscovered {} tools:", tools.len());
    for tool in &tools {
        println!("  - {} (from {})", tool.tool.name, tool.server_name);
        if let Some(desc) = &tool.tool.description {
            println!("    {}", desc);
        }
    }

    // Try reading a file
    println!("\nTesting read_file tool...");
    
    // Create a test file
    std::fs::write("/tmp/bosun-test.txt", "Hello from Bosun!")?;
    
    // Call the tool (without policy for this test)
    let policy = policy::Policy::permissive();
    let result = host
        .call_tool(
            "read_file",
            Some(serde_json::json!({ "path": "/tmp/bosun-test.txt" })),
            &policy,
        )
        .await;

    match result {
        Ok(r) => {
            println!("Success! Tool returned:");
            for content in &r.content {
                if let Some(text) = content.as_text() {
                    println!("  {}", text);
                }
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    // Cleanup
    std::fs::remove_file("/tmp/bosun-test.txt").ok();
    host.shutdown().await;

    println!("\nDone!");
    Ok(())
}
