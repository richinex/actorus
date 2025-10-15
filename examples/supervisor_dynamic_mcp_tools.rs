//! Dynamic MCP Tool Integration
//!
//! This demonstrates automatically discovering and using tools from ANY MCP server.
//! The library handles all the complexity - you just call discover_mcp_tools()
//! and get back ready-to-use tools for your agents.
//!
//! This is the power of MCP: plug in any server, get instant tool access!
//!
//! Prerequisites:
//! 1. npm install -g @modelcontextprotocol/server-brave-search
//! 2. npm install -g @modelcontextprotocol/server-filesystem (optional)
//! 3. export BRAVE_API_KEY=your_api_key_here

use actorus::core::mcp::discover_mcp_tools;
use actorus::{init, supervisor, AgentBuilder, AgentCollection};
use anyhow::Result;

// ============================================================================
// Main - Dynamic MCP Integration
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    println!("\n");
    println!("   DYNAMIC MCP TOOL INTEGRATION                               ");
    println!("\n");

    // Check for API key (for Brave Search)
    if std::env::var("BRAVE_API_KEY").is_err() {
        eprintln!("WARNING: BRAVE_API_KEY not set. Brave Search tools will fail.\n");
        eprintln!("Get your API key: https://brave.com/search/api/");
        eprintln!("Set it: export BRAVE_API_KEY=your_key_here\n");
        eprintln!("Continuing anyway to demonstrate tool discovery...\n");
    }

    init().await?;

    // ========================================================================
    // Discover Tools from Brave Search MCP Server
    // ========================================================================

    println!("");
    println!("   Step 1: Discover Brave Search Tools                       ");
    println!("\n");

    let brave_tools = discover_mcp_tools(
        "npx",
        vec!["-y", "@modelcontextprotocol/server-brave-search"],
    )
    .await?;

    println!(
        "\n Discovered {} tools from Brave Search MCP server\n",
        brave_tools.len()
    );

    // ========================================================================
    // Build Agent with Dynamically Discovered Tools
    // ========================================================================

    println!("");
    println!("   Step 2: Create Agent with MCP Tools                       ");
    println!("\n");

    let mut research_agent = AgentBuilder::new("research_agent")
        .description("Research agent with dynamically discovered MCP tools")
        .system_prompt(
            "You are a research agent with access to web search tools. \
             Use the available tools to answer questions with real-time information. \
             Be specific about which tool you're using and why.",
        );

    // Add all discovered tools
    for tool in brave_tools {
        research_agent = research_agent.tool_arc(tool);
    }

    println!(
        " Created agent with {} MCP tools\n",
        research_agent.tool_count()
    );

    // ========================================================================
    // Optional: Add More MCP Servers
    // ========================================================================

    // You could add filesystem server:
    // let fs_tools = discover_mcp_tools(
    //     "npx",
    //     vec!["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
    // ).await?;

    // Or GitHub server:
    // let github_tools = discover_mcp_tools(
    //     "npx",
    //     vec!["-y", "@modelcontextprotocol/server-github"],
    // ).await?;

    // ========================================================================
    // Execute Research Task
    // ========================================================================

    println!("");
    println!("   Step 3: Execute Research Task                             ");
    println!("\n");

    let agents = AgentCollection::new().add(research_agent);

    let research_task = "
        Research the latest developments in Rust programming language for 2025.

        Use the available search tools to find:
        1. Recent news and announcements
        2. Key technical developments
        3. Community trends

        Provide a concise summary with sources.
    ";

    println!("Task: Research Rust 2025 developments\n");
    println!("Agent working...\n");

    let result = supervisor::orchestrate_custom_agents(agents.build(), research_task).await?;

    println!("\n");
    println!("                    RESULTS                                   ");
    println!("\n");

    println!("Success: {}\n", result.success);
    println!("Result:\n{}\n", result.result);

    println!("Steps taken: {}", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        if let Some(action) = &step.action {
            println!("   {}. {}", i + 1, action);
        }
    }

    println!("\n");
    println!("        DYNAMIC MCP INTEGRATION COMPLETE                      ");
    println!("\n");

    println!("Next Steps:");
    println!("  - Install more MCP servers (filesystem, github, puppeteer)");
    println!("  - Create multi-server agents with diverse capabilities");
    println!("  - Build tool discovery UI");
    println!("  - Implement MCP server marketplace\n");

    Ok(())
}
