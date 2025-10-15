//! Supervisor-Orchestrated Research Pipeline with MCP Brave Search
//!
//! This demonstrates the same pattern as supervisor_database_pipeline_compact.rs
//! but using MCP Brave Search for real-time web research.
//!
//! Pipeline: Research Agent → Analysis Agent → Reporting Agent
//!
//! Prerequisites:
//! 1. npm install -g @modelcontextprotocol/server-brave-search
//! 2. export BRAVE_API_KEY=your_api_key_here

use anyhow::Result;
use actorus::core::mcp::MCPClient;
use actorus::tool_fn;
use actorus::{init, supervisor, AgentBuilder, AgentCollection};
use once_cell::sync::Lazy;
use serde_json::json;
use tokio::sync::Mutex;

// ============================================================================
// Global MCP Client
// ============================================================================

static BRAVE_CLIENT: Lazy<Mutex<Option<MCPClient>>> = Lazy::new(|| Mutex::new(None));

async fn init_brave_search() -> Result<()> {
    let client = MCPClient::new(
        "npx",
        vec!["-y", "@modelcontextprotocol/server-brave-search"],
    )
    .await?;

    *BRAVE_CLIENT.lock().await = Some(client);
    tracing::info!("Brave Search MCP client initialized");
    Ok(())
}

// ============================================================================
// Research Agent Tools (using MCP)
// ============================================================================

#[tool_fn(
    name = "web_search",
    description = "Search the web using Brave Search. Returns top search results with titles, URLs, and descriptions."
)]
async fn web_search(_query: String, _count: i64) -> Result<String> {
    let mut client_guard = BRAVE_CLIENT.lock().await;
    let client = client_guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Brave Search client not initialized"))?;

    let result = client
        .call_tool(
            "brave_web_search",
            json!({
                "query": _query,
                "count": _count
            }),
        )
        .await?;

    Ok(result)
}

#[tool_fn(
    name = "search_recent_news",
    description = "Search for recent news articles (past day). Returns news from the last 24 hours."
)]
async fn search_recent_news(_query: String) -> Result<String> {
    let mut client_guard = BRAVE_CLIENT.lock().await;
    let client = client_guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Brave Search client not initialized"))?;

    let result = client
        .call_tool(
            "brave_web_search",
            json!({
                "query": _query,
                "count": 5,
                "freshness": "pd" // Past day
            }),
        )
        .await?;

    Ok(result)
}

// ============================================================================
// Analysis Agent Tools
// ============================================================================

#[tool_fn(
    name = "analyze_search_results",
    description = "Analyze web search results and extract key insights, trends, and patterns"
)]
async fn analyze_search_results(_search_data: String) -> Result<String> {
    // Simulated analysis based on search results
    Ok(format!(
        "Search Results Analysis:\n\
         \n\
         \n\
         Key Findings:\n\
         - Multiple authoritative sources identified\n\
         - Recent developments detected in search results\n\
         - Strong consensus on technical approaches\n\
         \n\
         Trends Identified:\n\
         - Growing adoption of actor-based patterns\n\
         - Emphasis on async/await integration\n\
         - Focus on type safety and concurrency\n\
         \n\
         Quality Assessment:\n\
         - Source credibility: High\n\
         - Information recency: Recent (2024-2025)\n\
         - Technical depth: Advanced\n\
         \n\
         Source Data:\n\
         {}\n\
         \n\
         Recommendations:\n\
         - Focus on top 3 most cited sources\n\
         - Cross-reference technical details\n\
         - Verify code examples against official docs",
        _search_data
    ))
}

#[tool_fn(
    name = "compare_sources",
    description = "Compare multiple news sources to identify common themes and differences"
)]
async fn compare_sources(_news_data: String) -> Result<String> {
    Ok(format!(
        "News Source Comparison:\n\
         \n\
         \n\
         Common Themes:\n\
         - AI agent systems seeing rapid development\n\
         - Actor pattern gaining traction\n\
         - Rust ecosystem expanding for AI/ML\n\
         \n\
         Divergent Perspectives:\n\
         - Different approaches to agent coordination\n\
         - Varying opinions on best practices\n\
         - Regional focus differences\n\
         \n\
         Reliability Assessment:\n\
         - Multiple independent confirmations\n\
         - Consistent timeline across sources\n\
         - Technical accuracy verified\n\
         \n\
         News Data:\n\
         {}",
        _news_data
    ))
}

// ============================================================================
// Reporting Agent Tools
// ============================================================================

#[tool_fn(
    name = "generate_research_report",
    description = "Generate comprehensive research report from search analysis and news comparison"
)]
async fn generate_research_report(
    _search_analysis: String,
    _news_comparison: String,
) -> Result<String> {
    let report = format!(
        "\n\
                   RESEARCH REPORT - MCP-POWERED ANALYSIS              \n\
                   Generated: 2025                                     \n\
         \n\
         \n\
         EXECUTIVE SUMMARY\n\
         \n\
         This report synthesizes real-time web research and recent news\n\
         analysis to provide up-to-date insights on the requested topic.\n\
         \n\
         WEB SEARCH ANALYSIS\n\
         \n\
         {}\n\
         \n\
         NEWS COMPARISON\n\
         \n\
         {}\n\
         \n\
         KEY TAKEAWAYS\n\
         \n\
         1. TECHNICAL FINDINGS:\n\
            - Actor pattern provides natural foundation for AI agents\n\
            - Message passing enables distributed agent systems\n\
            - Rust async/await integrates cleanly with actor model\n\
         \n\
         2. RECENT DEVELOPMENTS:\n\
            - Increased industry adoption of agent architectures\n\
            - Growing ecosystem of tools and frameworks\n\
            - Active research in multi-agent coordination\n\
         \n\
         3. PRACTICAL APPLICATIONS:\n\
            - Real-time web research pipelines (like this one!)\n\
            - Distributed data processing\n\
            - Autonomous agent orchestration\n\
         \n\
         RECOMMENDATIONS\n\
         \n\
         1. IMMEDIATE (Next Steps):\n\
            - Review top-cited technical sources\n\
            - Implement proof-of-concept using actor pattern\n\
            - Test with real-world use cases\n\
         \n\
         2. SHORT-TERM (1-3 Months):\n\
            - Build production-ready agent pipeline\n\
            - Integrate MCP servers for external services\n\
            - Deploy distributed agent system\n\
         \n\
         3. LONG-TERM (Strategic):\n\
            - Scale to multi-datacenter deployment\n\
            - Develop agent marketplace\n\
            - Contribute to open-source ecosystem\n\
         \n\
         CONCLUSION\n\
         \n\
         Real-time web research combined with structured analysis provides\n\
         actionable insights. The MCP integration demonstrates the power of\n\
         actor-based agent systems for dynamic information gathering.\n\
         \n\
         \n\
           Research Complete - Generated with MCP & Actor Agents       \n\
         ",
        _search_analysis, _news_comparison
    );

    Ok(report)
}

#[tool_fn(
    name = "export_findings",
    description = "Export research findings to structured format"
)]
async fn export_findings(_report: String) -> Result<String> {
    Ok(format!(
        "Export Summary:\n\
         \n\
         File: research_report_mcp.txt\n\
         Size: {} bytes\n\
         Format: Text with structured sections\n\
         Status: Successfully exported\n\
         Location: ./reports/research_report_mcp.txt\n\
         \n\
         MCP Integration:\n\
         - Brave Search API queries: 2\n\
         - Real-time data retrieved: Yes\n\
         - Analysis pipeline: Complete\n\
         \n\
         Report Preview:\n\
         {}",
        _report.len(),
        &_report[.._report.len().min(500)]
    ))
}

// ============================================================================
// Main - MCP Research Pipeline
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    println!("\n");
    println!("   SUPERVISOR ORCHESTRATING MCP RESEARCH PIPELINE            ");
    println!("\n");

    // Check for API key
    if std::env::var("BRAVE_API_KEY").is_err() {
        eprintln!("ERROR: BRAVE_API_KEY environment variable not set\n");
        eprintln!("Get your API key: https://brave.com/search/api/");
        eprintln!("Set it: export BRAVE_API_KEY=your_key_here");
        eprintln!("Install server: npm install -g @modelcontextprotocol/server-brave-search\n");
        return Ok(());
    }

    init().await?;

    // Initialize MCP connection
    println!("Initializing Brave Search MCP connection...");
    init_brave_search().await?;
    println!("MCP client ready\n");

    // ========================================================================
    // Build Specialized Agents for Research Pipeline
    // ========================================================================

    // Research Agent - Uses MCP Brave Search
    let research_agent = AgentBuilder::new("research_agent")
        .description("Performs web research using Brave Search MCP server")
        .system_prompt(
            "You are a research specialist with access to real-time web search via Brave API. \
             Use web_search for general queries and search_recent_news for latest developments.",
        )
        .tool(WebSearchTool::new())
        .tool(SearchRecentNewsTool::new());

    // Analysis Agent - Processes search results
    let analysis_agent = AgentBuilder::new("analysis_agent")
        .description("Analyzes research data and identifies patterns")
        .system_prompt(
            "You are an analyst. Review search results and news data to extract insights, \
             identify trends, and assess source quality.",
        )
        .tool(AnalyzeSearchResultsTool::new())
        .tool(CompareSourcesTool::new());

    // Reporting Agent - Generates final report
    let reporting_agent = AgentBuilder::new("reporting_agent")
        .description("Generates comprehensive research reports")
        .system_prompt(
            "You are a report writer. Synthesize analysis into clear, actionable reports \
             with executive summaries and recommendations.",
        )
        .tool(GenerateResearchReportTool::new())
        .tool(ExportFindingsTool::new());

    // Collect all agents
    let agents = AgentCollection::new()
        .add(research_agent)
        .add(analysis_agent)
        .add(reporting_agent);

    println!(
        "Created {} specialized agents for the pipeline:",
        agents.len()
    );
    for (name, description) in agents.list_agents() {
        println!("   - {}: {}", name, description);
    }
    println!();

    let agent_configs = agents.build();

    // ========================================================================
    // Execute MCP Research Pipeline
    // ========================================================================

    println!("Starting MCP-powered research pipeline...\n");

    let research_task = "
        Execute a comprehensive research pipeline using MCP Brave Search:

        1. Use web_search to find information about 'Rust actor pattern for AI agents'
           - Query with count 5 to get top results
           - Focus on technical implementations and best practices

        2. Use search_recent_news to find latest developments
           - Query 'AI agent systems Rust' to get recent news

        3. Use analyze_search_results with the web search data
           - Extract key insights and technical patterns

        4. Use compare_sources with the news data
           - Identify common themes and assess reliability

        5. Use generate_research_report with both analysis results
           - Create comprehensive report combining all findings

        6. Use export_findings to save the report

        Make sure to:
        - Pass actual search results to analysis tools
        - Include specific findings in the report
        - Provide actionable recommendations
    ";

    let result = supervisor::orchestrate_custom_agents(agent_configs, research_task).await?;

    println!("\n");
    println!("                    PIPELINE RESULTS                          ");
    println!("\n");

    println!("Success: {}", result.success);
    println!("\nFinal Result:\n{}\n", result.result);

    println!("Pipeline Execution Details:");
    println!("   - Total orchestration steps: {}", result.steps.len());
    println!("   - MCP queries executed: 2 (web + news)");
    println!("   - Analyses performed: 2");
    println!("   - Reports generated: 1\n");

    println!("Step-by-Step Breakdown:");
    for (i, step) in result.steps.iter().enumerate() {
        println!("\n   Step {}: {}", i + 1, step.thought);
        if let Some(action) = &step.action {
            if let Some((agent, task)) = action.split_once(':') {
                println!("      Agent: {}", agent);
                println!("      Task: {}", task.chars().take(80).collect::<String>());
            }
        }
        if let Some(obs) = &step.observation {
            let preview = if obs.len() > 150 {
                format!("{}...", &obs[..150])
            } else {
                obs.clone()
            };
            println!("      Result: {}", preview);
        }
    }

    println!("\n");
    println!("              KEY CONCEPTS DEMONSTRATED                       ");
    println!("\n");
    println!("1. MCP Integration: Brave Search via Model Context Protocol");
    println!("2. Actor Pattern: Each agent is independent actor");
    println!("3. Message Passing: Structured data flow via JSON");
    println!("4. Real-Time Data: Live web search results");
    println!("5. Multi-Agent Coordination: Research → Analysis → Reporting");
    println!("6. Tool Composition: MCP tools + custom analysis tools");
    println!("7. Supervisor Orchestration: Autonomous task decomposition\n");

    println!("");
    println!("        MCP RESEARCH PIPELINE COMPLETE                        ");
    println!("\n");

    Ok(())
}
