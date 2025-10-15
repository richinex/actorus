//! Compact Database Analysis Pipeline Example
//!
//! This demonstrates a 3-agent pipeline coordinated by supervisor:
//! Database Agent → Analysis Agent → Reporting Agent
//!
//! Key concepts:
//! - Multi-agent orchestration
//! - Tool composition and data flow
//! - Business intelligence workflow

use actorus::tool_fn;
use actorus::{init, supervisor, AgentBuilder, AgentCollection};
use anyhow::Result;
use once_cell::sync::Lazy;
use rusqlite::Connection;
use serde::Serialize;
use std::sync::Mutex;

// ============================================================================
// Database Setup with Sample Sales Data
// ============================================================================

static DB: Lazy<Mutex<Connection>> = Lazy::new(|| {
    let conn = Connection::open_in_memory().expect("Failed to create database");
    conn.execute(
        "CREATE TABLE sales (
            product TEXT, region TEXT, quantity INTEGER, price REAL)",
        [],
    )
    .unwrap();

    // Sample data: 3 products, 4 regions
    let data = vec![
        ("Laptop", "North", 45, 1299.99),
        ("Laptop", "South", 32, 1299.99),
        ("Phone", "North", 120, 899.99),
        ("Phone", "South", 95, 899.99),
        ("Tablet", "North", 67, 499.99),
        ("Tablet", "South", 45, 499.99),
    ];

    for (product, region, qty, price) in data {
        conn.execute(
            "INSERT INTO sales VALUES (?1, ?2, ?3, ?4)",
            &[
                &product as &dyn rusqlite::ToSql,
                &region,
                &qty.to_string(),
                &price.to_string(),
            ],
        )
        .unwrap();
    }

    Mutex::new(conn)
});

// ============================================================================
// Database Agent Tools
// ============================================================================

#[tool_fn(name = "query_revenue", description = "Query total revenue by product")]
async fn query_revenue() -> Result<String> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT product, SUM(quantity * price) as revenue
         FROM sales GROUP BY product ORDER BY revenue DESC",
    )?;

    let mut results = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    for row in rows {
        let (product, revenue) = row?;
        results.push(format!("{}: ${:.2}", product, revenue));
    }

    Ok(format!("Revenue Analysis:\n{}", results.join("\n")))
}

#[tool_fn(name = "query_regions", description = "Query performance by region")]
async fn query_regions() -> Result<String> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT region, SUM(quantity * price) as revenue
         FROM sales GROUP BY region ORDER BY revenue DESC",
    )?;

    let mut results = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    for row in rows {
        let (region, revenue) = row?;
        results.push(format!("{}: ${:.2}", region, revenue));
    }

    Ok(format!("Regional Performance:\n{}", results.join("\n")))
}

// ============================================================================
// Analysis Agent Tools
// ============================================================================

#[tool_fn(
    name = "analyze_data",
    description = "Analyze sales data and generate insights"
)]
async fn analyze_data(_revenue_data: String, _region_data: String) -> Result<String> {
    Ok(format!(
        "Business Insights:\n\
         \n\
         Product Analysis:\n\
         {}\n\
         - High-value products driving revenue\n\
         - Strong product diversification\n\
         \n\
         Regional Analysis:\n\
         {}\n\
         - Geographic distribution shows opportunities\n\
         - Target underperforming regions for growth\n\
         \n\
         Recommendations:\n\
         - Focus marketing on top products\n\
         - Expand in weak regions\n\
         - Consider regional pricing strategies",
        _revenue_data, _region_data
    ))
}

// ============================================================================
// Reporting Agent Tools
// ============================================================================

#[tool_fn(name = "generate_report", description = "Generate executive summary")]
async fn generate_report(_analysis: String) -> Result<String> {
    Ok(format!(
        "\n\
               EXECUTIVE SUMMARY - Q1 2024            \n\
         \n\
         \n\
         {}\n\
         \n\
         STRATEGIC ACTIONS:\n\
         1. Launch regional campaigns (30 days)\n\
         2. Optimize product mix (90 days)\n\
         3. Expand sales team (12 months)\n\
         \n\
         Status: Analysis Complete ",
        _analysis
    ))
}

#[tool_fn(name = "export_json", description = "Export findings to JSON")]
async fn export_json(_report: String) -> Result<String> {
    #[derive(Serialize)]
    struct Export {
        report_type: String,
        status: String,
        size: usize,
    }

    let data = Export {
        report_type: "Sales Analysis".to_string(),
        status: "completed".to_string(),
        size: _report.len(),
    };

    let json = serde_json::to_string_pretty(&data)?;
    Ok(format!("Exported: sales_analysis.json\n{}", json))
}

// ============================================================================
// Main Pipeline
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("\n");
    println!("   DATABASE ANALYSIS PIPELINE                 ");
    println!("\n");

    init().await?;

    // Initialize database
    let _ = &*DB;
    println!("Database initialized with sales data\n");

    // Build specialized agents
    let database_agent = AgentBuilder::new("database_agent")
        .description("Executes SQL queries")
        .system_prompt(
            "You are a database specialist. Execute queries and return formatted results.",
        )
        .tool(QueryRevenueTool::new())
        .tool(QueryRegionsTool::new());
    let analysis_agent = AgentBuilder::new("analysis_agent")
        .description("Analyzes data and generates insights")
        .system_prompt("You are a business analyst. Analyze data and provide actionable insights.")
        .tool(AnalyzeDataTool::new());
    let reporting_agent = AgentBuilder::new("reporting_agent")
        .description("Generates reports")
        .system_prompt("You are a reporting specialist. Create executive summaries.")
        .tool(GenerateReportTool::new())
        .tool(ExportJsonTool::new());
    let agents = AgentCollection::new()
        .add(database_agent)
        .add(analysis_agent)
        .add(reporting_agent);

    let agent_configs = agents.build();

    // Execute pipeline
    let task = "
        Execute sales analysis pipeline:
        1. Query revenue data
        2. Query regional performance
        3. Analyze both datasets together
        4. Generate executive summary from analysis
        5. Export to JSON

        Pass data between steps for comprehensive analysis.
    ";
    let result = supervisor::orchestrate_custom_agents(agent_configs, task).await?;

    println!("\n");
    println!("            PIPELINE RESULTS                  ");
    println!("\n");

    println!("Success: {}\n", result.success);
    println!("Final Result:\n{}\n", result.result);

    println!("Execution Details:");
    println!("   - Steps: {}", result.steps.len());
    println!("   - Agents: Database → Analysis → Reporting\n");

    println!("Step Breakdown:");
    for (i, step) in result.steps.iter().enumerate() {
        println!("   {}. {}", i + 1, step.thought);
        if let Some(obs) = &step.observation {
            let preview = if obs.len() > 100 {
                format!("{}...", &obs[..100])
            } else {
                obs.clone()
            };
            println!("      Result: {}", preview);
        }
    }

    Ok(())
}
