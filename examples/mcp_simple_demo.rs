//! Simple MCP Brave Search Demo
//!
//! This shows basic usage of the Brave Search MCP server
//! without complex agent systems.
//!
//! Prerequisites:
//! 1. npm install -g @modelcontextprotocol/server-brave-search
//! 2. export BRAVE_API_KEY=your_api_key_here

use anyhow::Result;
use actorus::core::mcp::MCPClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("\n");
    println!("   BRAVE SEARCH MCP DEMO                      ");
    println!("\n");

    // Check for API key
    if std::env::var("BRAVE_API_KEY").is_err() {
        eprintln!("ERROR: BRAVE_API_KEY environment variable not set\n");
        eprintln!("Get your API key: https://brave.com/search/api/");
        eprintln!("Set it: export BRAVE_API_KEY=your_key_here\n");
        return Ok(());
    }

    println!("Connecting to Brave Search MCP server...\n");

    // Connect to Brave Search MCP server using npx
    let mut client = MCPClient::new(
        "npx",
        vec!["-y", "@modelcontextprotocol/server-brave-search"],
    )
    .await?;

    println!("Connected! Listing available tools...\n");

    // List available tools
    let tools = client.list_tools().await?;

    println!("Available Tools:");
    for tool in &tools {
        println!("  - {}", tool.name);
        if let Some(desc) = &tool.description {
            println!("    Description: {}", desc);
        }
    }
    println!();

    // Example 1: Simple web search
    println!("");
    println!("   Example 1: Web Search                     ");
    println!("\n");

    println!("Searching for: 'Rust actor pattern 2025'\n");

    let search_result = client
        .call_tool(
            "brave_web_search",
            json!({
                "query": "Rust actor pattern 2025",
                "count": 5
            }),
        )
        .await?;

    println!("Search Results:");
    println!("{}\n", search_result);

    // Example 2: Search with freshness filter
    println!("");
    println!("   Example 2: Recent News                    ");
    println!("\n");

    println!("Searching for recent news: 'AI agents'\n");

    let news_result = client
        .call_tool(
            "brave_web_search",
            json!({
                "query": "AI agents",
                "count": 3,
                "freshness": "pd" // Past day
            }),
        )
        .await?;

    println!("Recent Results:");
    println!("{}\n", news_result);

    println!("");
    println!("   MCP Demo Complete                         ");
    println!("\n");

    println!("What You've Seen:");
    println!("   Connected to external MCP server (Brave Search)");
    println!("   Listed available tools via JSON-RPC");
    println!("   Executed search queries");
    println!("   Got real-time web results\n");

    println!("Actor Pattern in Action:");
    println!("  - MCPClient spawns external process (npx)");
    println!("  - Communicates via stdin/stdout");
    println!("  - JSON-RPC protocol messages");
    println!("  - Async await for responses\n");

    Ok(())
}
