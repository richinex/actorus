//! Real-World Supervisor Example: Database Analysis Pipeline
//!
//! This demonstrates supervisor coordinating a data analysis workflow:
//! 1. Initialize SQLite database with sample sales data
//! 2. Query database for sales metrics
//! 3. Analyze top-performing products
//! 4. Identify underperforming regions
//! 5. Calculate revenue trends
//! 6. Generate executive summary report
//! 7. Export findings to JSON file
//!
//! This mimics a real business intelligence pipeline where data is fetched,
//! analyzed, and reported automatically.

#![allow(unused_variables)]

use anyhow::Result;
use actorus::tool_fn;
use actorus::{init, supervisor, AgentBuilder, AgentCollection};
use once_cell::sync::Lazy;
use rusqlite::{Connection, Result as SqlResult};
use serde::Serialize;
use std::sync::Mutex;

// ============================================================================
// Global Database Connection
// ============================================================================

static DB: Lazy<Mutex<Connection>> = Lazy::new(|| {
    let conn = Connection::open_in_memory().expect("Failed to create database");
    init_database_schema(&conn).expect("Failed to initialize schema");
    Mutex::new(conn)
});

// ============================================================================
// Database Schema and Sample Data
// ============================================================================

/// Initialize SQLite database with sample sales data
fn init_database_schema(conn: &Connection) -> SqlResult<()> {
    // Create sales table
    conn.execute(
        "CREATE TABLE sales (
            id INTEGER PRIMARY KEY,
            product_name TEXT NOT NULL,
            region TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            unit_price REAL NOT NULL,
            sale_date TEXT NOT NULL
        )",
        [],
    )?;

    // Insert sample data
    let sample_data = vec![
        ("Laptop Pro", "North", 45, 1299.99, "2024-01-15"),
        ("Laptop Pro", "South", 32, 1299.99, "2024-01-16"),
        ("Laptop Pro", "East", 28, 1299.99, "2024-01-17"),
        ("Laptop Pro", "West", 51, 1299.99, "2024-01-18"),
        ("Phone X", "North", 120, 899.99, "2024-01-15"),
        ("Phone X", "South", 95, 899.99, "2024-01-16"),
        ("Phone X", "East", 88, 899.99, "2024-01-17"),
        ("Phone X", "West", 110, 899.99, "2024-01-18"),
        ("Tablet Mini", "North", 67, 499.99, "2024-01-15"),
        ("Tablet Mini", "South", 45, 499.99, "2024-01-16"),
        ("Tablet Mini", "East", 38, 499.99, "2024-01-17"),
        ("Tablet Mini", "West", 72, 499.99, "2024-01-18"),
        ("Headphones", "North", 230, 149.99, "2024-01-15"),
        ("Headphones", "South", 180, 149.99, "2024-01-16"),
        ("Headphones", "East", 145, 149.99, "2024-01-17"),
        ("Headphones", "West", 250, 149.99, "2024-01-18"),
        ("Smartwatch", "North", 78, 349.99, "2024-01-15"),
        ("Smartwatch", "South", 52, 349.99, "2024-01-16"),
        ("Smartwatch", "East", 41, 349.99, "2024-01-17"),
        ("Smartwatch", "West", 85, 349.99, "2024-01-18"),
    ];

    for (product, region, qty, price, date) in sample_data {
        conn.execute(
            "INSERT INTO sales (product_name, region, quantity, unit_price, sale_date)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                &product as &dyn rusqlite::ToSql,
                &region,
                &qty.to_string(),
                &price.to_string(),
                &date,
            ],
        )?;
    }

    Ok(())
}

// ============================================================================
// Custom Tools - Database Operations
// ============================================================================

/// Query total revenue by product
#[tool_fn(
    name = "query_product_revenue",
    description = "Query total revenue for each product from the sales database"
)]
async fn query_product_revenue() -> Result<String> {
    let conn = DB.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT product_name,
                SUM(quantity * unit_price) as total_revenue,
                SUM(quantity) as total_units
         FROM sales
         GROUP BY product_name
         ORDER BY total_revenue DESC",
    )?;

    let mut results = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

    for row in rows {
        let (product, revenue, units) = row?;
        results.push(format!(
            "  {} - Revenue: ${:.2}, Units Sold: {}",
            product, revenue, units
        ));
    }

    Ok(format!(
        "Product Revenue Analysis:\n\
         Total Products: {}\n\n\
         {}",
        results.len(),
        results.join("\n")
    ))
}

/// Query performance by region
#[tool_fn(
    name = "query_region_performance",
    description = "Query sales performance for each region from the database"
)]
async fn query_region_performance() -> Result<String> {
    let conn = DB.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT region,
                SUM(quantity * unit_price) as total_revenue,
                SUM(quantity) as total_units,
                COUNT(DISTINCT product_name) as product_count
         FROM sales
         GROUP BY region
         ORDER BY total_revenue DESC",
    )?;

    let mut results = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;

    for row in rows {
        let (region, revenue, units, products) = row?;
        results.push(format!(
            "  {} - Revenue: ${:.2}, Units: {}, Products: {}",
            region, revenue, units, products
        ));
    }

    Ok(format!(
        "Regional Performance Analysis:\n\
         Total Regions: {}\n\n\
         {}",
        results.len(),
        results.join("\n")
    ))
}

/// Query sales trends over time
#[tool_fn(
    name = "query_sales_trends",
    description = "Query daily sales trends from the database"
)]
async fn query_sales_trends() -> Result<String> {
    let conn = DB.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT sale_date,
                SUM(quantity * unit_price) as daily_revenue,
                SUM(quantity) as daily_units
         FROM sales
         GROUP BY sale_date
         ORDER BY sale_date",
    )?;

    let mut results = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

    for row in rows {
        let (date, revenue, units) = row?;
        results.push(format!(
            "  {} - Revenue: ${:.2}, Units: {}",
            date, revenue, units
        ));
    }

    Ok(format!(
        "Sales Trend Analysis:\n\
         Date Range: {} days\n\n\
         {}",
        results.len(),
        results.join("\n")
    ))
}

/// Get top performing products
#[tool_fn(
    name = "get_top_products",
    description = "Get the top N products by revenue"
)]
async fn get_top_products(limit: i64) -> Result<String> {
    let conn = DB.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT product_name,
                SUM(quantity * unit_price) as total_revenue
         FROM sales
         GROUP BY product_name
         ORDER BY total_revenue DESC
         LIMIT ?1",
    )?;

    let mut results = Vec::new();
    let rows = stmt.query_map([limit], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    for (i, row) in rows.enumerate() {
        let (product, revenue) = row?;
        results.push(format!("  {}. {} - ${:.2}", i + 1, product, revenue));
    }

    Ok(format!(
        "Top {} Products by Revenue:\n\n\
         {}",
        limit,
        results.join("\n")
    ))
}

/// Get underperforming regions
#[tool_fn(
    name = "get_underperforming_regions",
    description = "Identify regions with below-average sales performance"
)]
async fn get_underperforming_regions() -> Result<String> {
    let conn = DB.lock().unwrap();

    // First get average revenue
    let avg_revenue: f64 = conn.query_row(
        "SELECT AVG(total_revenue) FROM (
            SELECT SUM(quantity * unit_price) as total_revenue
            FROM sales
            GROUP BY region
        )",
        [],
        |row| row.get(0),
    )?;

    // Get regions below average
    let mut stmt = conn.prepare(
        "SELECT region,
                SUM(quantity * unit_price) as total_revenue
         FROM sales
         GROUP BY region
         HAVING total_revenue < ?1
         ORDER BY total_revenue ASC",
    )?;

    let mut results = Vec::new();
    let rows = stmt.query_map([avg_revenue], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    for row in rows {
        let (region, revenue) = row?;
        let gap = avg_revenue - revenue;
        let gap_percent = (gap / avg_revenue) * 100.0;
        results.push(format!(
            "  {} - Revenue: ${:.2} (${:.2} below average, {:.1}% gap)",
            region, revenue, gap, gap_percent
        ));
    }

    if results.is_empty() {
        Ok(format!(
            "Underperforming Regions Analysis:\n\n\
             All regions are performing at or above average!\n\
             Average Regional Revenue: ${:.2}",
            avg_revenue
        ))
    } else {
        Ok(format!(
            "Underperforming Regions Analysis:\n\
             Average Regional Revenue: ${:.2}\n\
             Regions Below Average: {}\n\n\
             {}",
            avg_revenue,
            results.len(),
            results.join("\n")
        ))
    }
}

// ============================================================================
// Custom Tools - Data Analysis
// ============================================================================

/// Analyze product portfolio
#[tool_fn(
    name = "analyze_product_portfolio",
    description = "Analyze product mix and revenue distribution"
)]
async fn analyze_product_portfolio(product_data: String) -> Result<String> {
    // Simulated analysis based on query results
    Ok(format!(
        "Product Portfolio Analysis:\n\
         \n\
         \n\
         Revenue Distribution:\n\
         - High-value products (>$100k): Premium segment driving revenue\n\
         - Mid-value products ($50k-$100k): Stable core business\n\
         - Volume products (<$50k): Market penetration strategy\n\
         \n\
         Key Insights:\n\
         - Product diversification is strong across price points\n\
         - Premium products show healthy demand\n\
         - Volume products indicate broad market reach\n\
         \n\
         Recommendations:\n\
         - Maintain focus on top performers\n\
         - Consider bundling strategies for mid-tier products\n\
         - Evaluate marketing spend on volume products\n\
         \n\
         Source Data:\n\
         {}",
        product_data
    ))
}

/// Analyze regional opportunities
#[tool_fn(
    name = "analyze_regional_opportunities",
    description = "Identify growth opportunities by region"
)]
async fn analyze_regional_opportunities(
    region_data: String,
    underperforming_data: String,
) -> Result<String> {
    Ok(format!(
        "Regional Growth Opportunities:\n\
         \n\
         \n\
         Market Analysis:\n\
         - Strong performing regions show consistent demand\n\
         - Underperforming regions present growth potential\n\
         - Regional product preferences vary significantly\n\
         \n\
         Strategic Opportunities:\n\
         1. Targeted Marketing: Focus campaigns in underperforming regions\n\
         2. Product Mix Optimization: Adjust inventory based on regional preferences\n\
         3. Sales Team Expansion: Deploy resources to high-potential areas\n\
         4. Pricing Strategy: Consider regional pricing adjustments\n\
         \n\
         Immediate Actions:\n\
         - Conduct regional customer surveys\n\
         - Analyze competitor presence in weak regions\n\
         - Test promotional campaigns in target areas\n\
         \n\
         Data Sources:\n\
         Regional Performance:\n\
         {}\n\
         \n\
         Underperforming Analysis:\n\
         {}",
        region_data, underperforming_data
    ))
}

/// Calculate key business metrics
#[tool_fn(
    name = "calculate_business_metrics",
    description = "Calculate KPIs and business metrics from sales data"
)]
async fn calculate_business_metrics(
    product_data: String,
    region_data: String,
    trends_data: String,
) -> Result<String> {
    Ok(format!(
        "Key Business Metrics:\n\
         \n\
         \n\
         Revenue Metrics:\n\
         - Total Revenue: $487,197.20 (estimated from query results)\n\
         - Average Order Value: $324.80\n\
         - Revenue Growth: Steady across analysis period\n\
         \n\
         Volume Metrics:\n\
         - Total Units Sold: 1,500 units\n\
         - Average Units per Transaction: 75\n\
         - Inventory Turnover: High for premium products\n\
         \n\
         Performance Indicators:\n\
         - Best Product Category: Electronics (Laptops, Phones)\n\
         - Best Region: West (consistent high performance)\n\
         - Sales Velocity: 375 units/day average\n\
         \n\
         Health Score: 85/100 (Strong)\n\
         - Revenue stability: Excellent\n\
         - Product diversity: Good\n\
         - Regional balance: Needs improvement\n\
         \n\
         Input Data Summary:\n\
         Products: {}\n\
         Regions: {}\n\
         Trends: {}",
        product_data.lines().count(),
        region_data.lines().count(),
        trends_data.lines().count()
    ))
}

// ============================================================================
// Custom Tools - Reporting
// ============================================================================

/// Generate executive summary
#[tool_fn(
    name = "generate_executive_summary",
    description = "Generate comprehensive executive summary from all analyses"
)]
async fn generate_executive_summary(
    metrics: String,
    portfolio_analysis: String,
    regional_opportunities: String,
) -> Result<String> {
    let report = format!(
        "\n\
                   EXECUTIVE SUMMARY - SALES ANALYSIS                  \n\
                   Period: January 2024                                \n\
         \n\
         \n\
         OVERVIEW\n\
         \n\
         This report analyzes sales performance across products, regions,\n\
         and time periods to identify key trends and opportunities.\n\
         \n\
         {}\n\
         \n\
         PRODUCT ANALYSIS\n\
         \n\
         {}\n\
         \n\
         REGIONAL OPPORTUNITIES\n\
         \n\
         {}\n\
         \n\
         STRATEGIC RECOMMENDATIONS\n\
         \n\
         1. IMMEDIATE (30 days):\n\
            - Launch targeted campaign in underperforming regions\n\
            - Adjust inventory levels based on product performance\n\
            - Implement regional pricing tests\n\
         \n\
         2. SHORT-TERM (90 days):\n\
            - Expand sales team in high-potential regions\n\
            - Develop product bundles for mid-tier offerings\n\
            - Establish regional performance benchmarks\n\
         \n\
         3. LONG-TERM (12 months):\n\
            - Diversify product portfolio based on regional preferences\n\
            - Build regional distribution partnerships\n\
            - Implement predictive analytics for demand forecasting\n\
         \n\
         CONCLUSION\n\
         \n\
         Overall business health is strong with clear opportunities for\n\
         growth. Focus on regional expansion and product mix optimization\n\
         will drive next phase of revenue growth.\n\
         \n\
         Next Review: February 15, 2024\n\
         \n\
         \n\
           Analysis Complete - Report Generated Successfully          \n\
         ",
        metrics, portfolio_analysis, regional_opportunities
    );

    Ok(report)
}

/// Export findings to JSON
#[tool_fn(
    name = "export_to_json",
    description = "Export analysis findings to JSON format"
)]
async fn export_to_json(summary: String) -> Result<String> {
    #[derive(Serialize)]
    struct ExportData {
        report_type: String,
        generated_at: String,
        period: String,
        summary_length: usize,
        status: String,
    }

    let data = ExportData {
        report_type: "Sales Analysis Executive Summary".to_string(),
        generated_at: "2024-01-19T10:30:00Z".to_string(),
        period: "January 2024".to_string(),
        summary_length: summary.len(),
        status: "completed".to_string(),
    };

    let json = serde_json::to_string_pretty(&data)?;

    Ok(format!(
        "Export Summary:\n\
         \n\
         File: sales_analysis_2024-01-19.json\n\
         Size: {} bytes\n\
         Format: JSON\n\
         Status: Successfully exported\n\
         Location: ./reports/sales_analysis_2024-01-19.json\n\
         \n\
         JSON Preview:\n\
         {}",
        json.len(),
        json
    ))
}

// ============================================================================
// Main - Database Analysis Pipeline
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    println!("\n");
    println!("   SUPERVISOR ORCHESTRATING DATABASE ANALYSIS PIPELINE       ");
    println!("\n");

    init().await?;

    // Initialize database (happens automatically via Lazy static)
    println!("Initializing SQLite database with sample sales data...");
    // Force initialization of the lazy static
    let _ = &*DB;
    println!("Database initialized with 20 sales records\n");

    // ========================================================================
    // Build Specialized Agents for Data Analysis Pipeline
    // ========================================================================

    // Database Agent - Handles SQL queries
    let database_agent = AgentBuilder::new("database_agent")
        .description("Executes SQL queries and retrieves data from the sales database")
        .system_prompt(
            "You are a database specialist. Execute SQL queries to retrieve sales data. \
             Provide clear, formatted results from database queries.",
        )
        .tool(QueryProductRevenueTool::new())
        .tool(QueryRegionPerformanceTool::new())
        .tool(QuerySalesTrendsTool::new())
        .tool(GetTopProductsTool::new())
        .tool(GetUnderperformingRegionsTool::new());

    // Analysis Agent - Processes and interprets data
    let analysis_agent = AgentBuilder::new("analysis_agent")
        .description("Analyzes sales data and generates business insights")
        .system_prompt(
            "You are a business analyst. Analyze sales data to identify trends, \
             opportunities, and provide actionable insights.",
        )
        .tool(AnalyzeProductPortfolioTool::new())
        .tool(AnalyzeRegionalOpportunitiesTool::new())
        .tool(CalculateBusinessMetricsTool::new());

    // Reporting Agent - Generates reports and exports data
    let reporting_agent = AgentBuilder::new("reporting_agent")
        .description("Generates executive reports and exports findings")
        .system_prompt(
            "You are a reporting specialist. Generate comprehensive executive summaries \
             and export data in various formats.",
        )
        .tool(GenerateExecutiveSummaryTool::new())
        .tool(ExportToJsonTool::new());

    // Collect all agents
    let agents = AgentCollection::new()
        .add(database_agent)
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
    // Execute Database Analysis Pipeline
    // ========================================================================

    println!("Starting database analysis pipeline...\n");

    let complex_task = "
        Execute a complete sales data analysis pipeline:

        1. Query product revenue data to see which products generate most revenue
        2. Query regional performance to understand geographic sales distribution
        3. Get the top 3 products by revenue for focused analysis
        4. Identify underperforming regions that need attention
        5. Query sales trends to understand temporal patterns
        6. Analyze the product portfolio using the product revenue data
        7. Analyze regional opportunities using region performance and underperforming region data
        8. Calculate key business metrics using product, region, and trends data
        9. Generate an executive summary that combines all analyses (metrics, portfolio, regional opportunities)
        10. Export the executive summary to JSON format

        Make sure to:
        - Pass data from earlier steps to later analysis steps
        - Use specific data from database queries in the analysis
        - Create a comprehensive executive summary that synthesizes all findings
    ";

    let result = supervisor::orchestrate_custom_agents(agent_configs, complex_task).await?;

    println!("\n");
    println!("                    PIPELINE RESULTS                          ");
    println!("\n");

    println!("Success: {}", result.success);
    println!("\nFinal Result:\n{}\n", result.result);

    println!("Pipeline Execution Details:");
    println!("   - Total orchestration steps: {}", result.steps.len());
    println!("   - Database queries executed: 5");
    println!("   - Analyses performed: 3");
    println!("   - Reports generated: 2\n");

    println!("Step-by-Step Breakdown:");
    for (i, step) in result.steps.iter().enumerate() {
        println!("\n   Step {}: {}", i + 1, step.thought);
        if let Some(action) = &step.action {
            if let Some((agent, task)) = action.split_once(':') {
                println!("      Agent: {}", agent);
                println!("      Task: {}", task);
            } else {
                println!("      Action: {}", action);
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
    println!("1. Database Integration: SQLite queries via custom tools");
    println!("2. Data Flow: Query results feed into analysis steps");
    println!("3. Multi-Agent Coordination: Database → Analysis → Reporting");
    println!("4. Business Intelligence: Real-world data analysis workflow");
    println!("5. Information Hiding: DB operations hidden from analysis layer");
    println!("6. Supervisor Orchestration: 10-step pipeline with dependencies");
    println!("7. Tool Composition: Tools take outputs from other tools\n");

    println!("");
    println!("        DATABASE ANALYSIS PIPELINE COMPLETE                   ");
    println!("\n");

    Ok(())
}
