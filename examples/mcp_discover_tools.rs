//! MCP Tool Discovery Demo
//!
//! This demonstrates discovering tools from an MCP server.
//! No API keys needed - just shows what tools are available.
//!
//! Prerequisites:
//! 1. npm install -g @modelcontextprotocol/server-brave-search
//! 2. npm install -g @modelcontextprotocol/server-filesystem (optional)

use anyhow::Result;
use actorus::core::mcp::discover_mcp_tools;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    println!("\n");
    println!("   MCP TOOL DISCOVERY DEMONSTRATION                          ");
    println!("\n");

    // ========================================================================
    // Example 1: Discover Brave Search Tools
    // ========================================================================

    println!("");
    println!("   Brave Search MCP Server                                   ");
    println!("\n");

    match discover_mcp_tools(
        "npx",
        vec!["-y", "@modelcontextprotocol/server-brave-search"],
    )
    .await
    {
        Ok(tools) => {
            println!(" Discovered {} tools:\n", tools.len());
            for tool in tools {
                let metadata = tool.metadata();
                println!("  Tool: {}", metadata.name);
                println!("    Description: {}", metadata.description);
                if !metadata.parameters.is_empty() {
                    println!("    Parameters:");
                    for param in &metadata.parameters {
                        let req = if param.required { "required" } else { "optional" };
                        println!(
                            "      - {} ({}): {}",
                            param.name, req, param.description
                        );
                    }
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!(" Failed to discover Brave Search tools: {}", e);
            eprintln!("  Make sure you have installed:");
            eprintln!("  npm install -g @modelcontextprotocol/server-brave-search\n");
        }
    }

    // ========================================================================
    // Example 2: Try Filesystem Server (if available)
    // ========================================================================

    println!("");
    println!("   Filesystem MCP Server (Optional)                         ");
    println!("\n");

    match discover_mcp_tools(
        "npx",
        vec![
            "-y",
            "@modelcontextprotocol/server-filesystem",
            "/tmp",
        ],
    )
    .await
    {
        Ok(tools) => {
            println!(" Discovered {} tools:\n", tools.len());
            for tool in tools {
                let metadata = tool.metadata();
                println!("  Tool: {}", metadata.name);
                println!("    Description: {}", metadata.description);
                println!();
            }
        }
        Err(e) => {
            println!(" Filesystem server not available: {}", e);
            println!("  (This is optional - you can install it with:");
            println!("   npm install -g @modelcontextprotocol/server-filesystem)\n");
        }
    }

    // ========================================================================
    // Summary
    // ========================================================================

    println!("");
    println!("   KEY FEATURES DEMONSTRATED                                 ");
    println!("\n");

    println!("1. Dynamic Tool Discovery:");
    println!("   - Call discover_mcp_tools() with server command");
    println!("   - Get back Vec<Arc<dyn Tool>> ready to use");
    println!("   - No manual tool definitions needed\n");

    println!("2. Automatic Schema Extraction:");
    println!("   - JSON schema → ToolMetadata conversion");
    println!("   - Parameter names, types, descriptions");
    println!("   - Required vs optional parameters\n");

    println!("3. Plug-and-Play Integration:");
    println!("   - npm install new MCP server");
    println!("   - Instant access to all its tools");
    println!("   - Works with ANY MCP-compliant server\n");

    println!("4. Usage in Agents:");
    println!("   ```rust");
    println!("   let tools = discover_mcp_tools(\"npx\", vec![\"-y\", \"server\"]).await?;");
    println!("   let agent = AgentBuilder::new(\"my_agent\")");
    println!("       .tools(tools)  // Add all discovered tools");
    println!("       .build();");
    println!("   ```\n");

    println!("");
    println!("   TOOL DISCOVERY COMPLETE                                   ");
    println!("\n");

    Ok(())
}
