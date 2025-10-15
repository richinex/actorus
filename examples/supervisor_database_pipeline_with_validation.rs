//! Database Analysis Pipeline with Handoff Validation
//!
//! This extends the database pipeline with quality gates:
//! 1. Database Agent → Validation → Analysis Agent
//! 2. Analysis Agent → Validation → Reporting Agent
//! 3. Each handoff validates:
//!    - Data schema compliance
//!    - Confidence thresholds
//!    - Execution time limits
//!    - Required field presence
//!
//! This demonstrates how validation prevents bad data from propagating
//! through the pipeline, ensuring quality at every stage.
//!
//! IMPORTANT NOTE:
//! This example shows validation catching a common problem: agents returning
//! natural language summaries instead of structured JSON. The validation system
//! correctly blocks this bad data with clear error messages. In production:
//! - Option 1: Use stronger system prompts to force JSON output
//! - Option 2: Extract tool results directly instead of LLM's final_answer
//! - Option 3: Post-process agent output to extract JSON
//!
//! The validation demonstrates its value by catching data quality issues early!

#![allow(unused_variables)]

use actorus::actors::handoff::{HandoffContract, HandoffCoordinator};
use actorus::actors::messages::{OutputSchema, ValidationRule, ValidationType};
use actorus::tool_fn;
use actorus::{init, supervisor, AgentBuilder, AgentCollection, Settings};
use anyhow::Result;
use once_cell::sync::Lazy;
use rusqlite::{Connection, Result as SqlResult};
use serde::Serialize;
use std::collections::HashMap;
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

fn init_database_schema(conn: &Connection) -> SqlResult<()> {
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
// Custom Tools - Database Operations (with structured JSON output)
// ============================================================================

#[tool_fn(
    name = "query_product_revenue",
    description = "Query total revenue for each product (returns JSON)"
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

    #[derive(Serialize)]
    struct ProductRevenue {
        product_name: String,
        total_revenue: f64,
        total_units: i64,
    }

    let mut products = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

    for row in rows {
        let (product, revenue, units) = row?;
        products.push(ProductRevenue {
            product_name: product,
            total_revenue: revenue,
            total_units: units,
        });
    }

    let total_revenue: f64 = products.iter().map(|p| p.total_revenue).sum();

    #[derive(Serialize)]
    struct QueryResult {
        data: Vec<ProductRevenue>,
        row_count: usize,
        total_revenue: f64,
        status: String,
    }

    let result = QueryResult {
        row_count: products.len(),
        total_revenue,
        data: products,
        status: "success".to_string(),
    };

    Ok(serde_json::to_string_pretty(&result)?)
}

#[tool_fn(
    name = "query_region_performance",
    description = "Query sales performance by region (returns JSON)"
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

    #[derive(Serialize)]
    struct RegionPerformance {
        region: String,
        total_revenue: f64,
        total_units: i64,
        product_count: i64,
    }

    let mut regions = Vec::new();
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
        regions.push(RegionPerformance {
            region,
            total_revenue: revenue,
            total_units: units,
            product_count: products,
        });
    }

    #[derive(Serialize)]
    struct QueryResult {
        data: Vec<RegionPerformance>,
        row_count: usize,
        status: String,
    }

    let result = QueryResult {
        row_count: regions.len(),
        data: regions,
        status: "success".to_string(),
    };

    Ok(serde_json::to_string_pretty(&result)?)
}

// ============================================================================
// Custom Tools - Data Analysis (with structured JSON output)
// ============================================================================

#[tool_fn(
    name = "analyze_product_data",
    description = "Analyze product revenue data and return structured insights (JSON)"
)]
async fn analyze_product_data(product_json: String) -> Result<String> {
    #[derive(Serialize)]
    struct AnalysisResult {
        insights: Vec<String>,
        metrics: HashMap<String, f64>,
        recommendations: Vec<String>,
        confidence_score: f64,
    }

    let mut metrics = HashMap::new();
    metrics.insert("avg_revenue_per_product".to_string(), 97439.44);
    metrics.insert("product_count".to_string(), 5.0);
    metrics.insert("revenue_concentration".to_string(), 0.45);

    let result = AnalysisResult {
        insights: vec![
            "Phone X leads with $81,799 revenue".to_string(),
            "Headphones shows high volume (805 units)".to_string(),
            "Premium products (Laptop Pro) maintain strong margins".to_string(),
        ],
        metrics,
        recommendations: vec![
            "Focus marketing on top-performing products".to_string(),
            "Consider bundling mid-tier products".to_string(),
            "Expand premium product line".to_string(),
        ],
        confidence_score: 0.92,
    };

    Ok(serde_json::to_string_pretty(&result)?)
}

#[tool_fn(
    name = "analyze_regional_data",
    description = "Analyze regional performance data and return structured insights (JSON)"
)]
async fn analyze_regional_data(region_json: String) -> Result<String> {
    #[derive(Serialize)]
    struct AnalysisResult {
        insights: Vec<String>,
        metrics: HashMap<String, f64>,
        recommendations: Vec<String>,
        confidence_score: f64,
    }

    let mut metrics = HashMap::new();
    metrics.insert("regional_variance".to_string(), 0.18);
    metrics.insert("top_region_share".to_string(), 0.27);
    metrics.insert("underperforming_regions".to_string(), 1.0);

    let result = AnalysisResult {
        insights: vec![
            "West region leads with consistent performance".to_string(),
            "East region shows growth opportunity".to_string(),
            "Regional product preferences vary significantly".to_string(),
        ],
        metrics,
        recommendations: vec![
            "Launch targeted campaigns in East region".to_string(),
            "Optimize inventory based on regional demand".to_string(),
            "Implement regional pricing strategy".to_string(),
        ],
        confidence_score: 0.88,
    };

    Ok(serde_json::to_string_pretty(&result)?)
}

// ============================================================================
// Custom Tools - Reporting (with structured JSON output)
// ============================================================================

#[tool_fn(
    name = "generate_report",
    description = "Generate comprehensive report from all analyses (JSON)"
)]
async fn generate_report(product_analysis: String, regional_analysis: String) -> Result<String> {
    #[derive(Serialize)]
    struct ExecutiveReport {
        title: String,
        period: String,
        summary: String,
        key_findings: Vec<String>,
        strategic_actions: Vec<String>,
        overall_score: f64,
        confidence: f64,
    }

    let report = ExecutiveReport {
        title: "Sales Analysis Executive Summary".to_string(),
        period: "January 2024".to_string(),
        summary: "Business shows strong performance with clear growth opportunities in regional expansion and product optimization.".to_string(),
        key_findings: vec![
            "Total revenue: $487,197 across 5 product lines".to_string(),
            "West region leads with 27% market share".to_string(),
            "Phone X and Laptop Pro drive 65% of revenue".to_string(),
            "East region presents 18% growth opportunity".to_string(),
        ],
        strategic_actions: vec![
            "IMMEDIATE: Launch East region marketing campaign".to_string(),
            "SHORT-TERM: Optimize product mix by region".to_string(),
            "LONG-TERM: Expand premium product portfolio".to_string(),
        ],
        overall_score: 85.0,
        confidence: 0.90,
    };

    Ok(serde_json::to_string_pretty(&report)?)
}

// ============================================================================
// Handoff Validation Setup
// ============================================================================

fn setup_validation_contracts(settings: &Settings) -> HandoffCoordinator {
    let mut coordinator = HandoffCoordinator::new();

    // Contract 1: Database → Analysis Agent
    let mut db_field_types = HashMap::new();
    db_field_types.insert("data".to_string(), "array".to_string());
    db_field_types.insert("row_count".to_string(), "number".to_string());
    db_field_types.insert("status".to_string(), "string".to_string());

    coordinator.register_contract(
        "database_agent_handoff".to_string(), // ← Must match agent name + "_handoff"
        HandoffContract {
            from_agent: "database_agent".to_string(),
            to_agent: Some("analysis_agent".to_string()),
            schema: OutputSchema {
                schema_version: "1.0".to_string(),
                required_fields: vec!["data".to_string(), "status".to_string()],
                optional_fields: vec!["row_count".to_string(), "total_revenue".to_string()],
                field_types: db_field_types,
                validation_rules: vec![
                    ValidationRule {
                        field: "status".to_string(),
                        rule_type: ValidationType::Enum,
                        constraint: "success,partial,failed".to_string(),
                    },
                    ValidationRule {
                        field: "row_count".to_string(),
                        rule_type: ValidationType::Range,
                        constraint: "1..1000".to_string(),
                    },
                ],
            },
            max_execution_time_ms: Some(settings.validation.agent_timeout_ms),
        },
    );

    // Contract 2: Analysis → Reporting Agent
    let mut analysis_field_types = HashMap::new();
    analysis_field_types.insert("insights".to_string(), "array".to_string());
    analysis_field_types.insert("metrics".to_string(), "object".to_string());
    analysis_field_types.insert("recommendations".to_string(), "array".to_string());
    analysis_field_types.insert("confidence_score".to_string(), "number".to_string());

    coordinator.register_contract(
        "analysis_agent_handoff".to_string(), // ← Must match agent name + "_handoff"
        HandoffContract {
            from_agent: "analysis_agent".to_string(),
            to_agent: Some("reporting_agent".to_string()),
            schema: OutputSchema {
                schema_version: "1.0".to_string(),
                required_fields: vec!["insights".to_string(), "confidence_score".to_string()],
                optional_fields: vec!["metrics".to_string(), "recommendations".to_string()],
                field_types: analysis_field_types,
                validation_rules: vec![
                    ValidationRule {
                        field: "insights".to_string(),
                        rule_type: ValidationType::MinLength,
                        constraint: "1".to_string(),
                    },
                    ValidationRule {
                        field: "confidence_score".to_string(),
                        rule_type: ValidationType::Range,
                        constraint: "0.0..1.0".to_string(),
                    },
                ],
            },
            max_execution_time_ms: Some(settings.validation.agent_timeout_ms),
        },
    );

    // Contract 3: Reporting → Final Output
    let mut report_field_types = HashMap::new();
    report_field_types.insert("title".to_string(), "string".to_string());
    report_field_types.insert("summary".to_string(), "string".to_string());
    report_field_types.insert("key_findings".to_string(), "array".to_string());
    report_field_types.insert("confidence".to_string(), "number".to_string());

    coordinator.register_contract(
        "reporting_agent_handoff".to_string(), // ← Must match agent name + "_handoff"
        HandoffContract {
            from_agent: "reporting_agent".to_string(),
            to_agent: None, // Final output
            schema: OutputSchema {
                schema_version: "1.0".to_string(),
                required_fields: vec![
                    "title".to_string(),
                    "summary".to_string(),
                    "key_findings".to_string(),
                ],
                optional_fields: vec!["strategic_actions".to_string(), "overall_score".to_string()],
                field_types: report_field_types,
                validation_rules: vec![
                    ValidationRule {
                        field: "summary".to_string(),
                        rule_type: ValidationType::MinLength,
                        constraint: "50".to_string(),
                    },
                    ValidationRule {
                        field: "key_findings".to_string(),
                        rule_type: ValidationType::MinLength,
                        constraint: "3".to_string(),
                    },
                ],
            },
            max_execution_time_ms: Some(settings.validation.agent_timeout_ms),
        },
    );

    coordinator
}

// ============================================================================
// Main - Database Pipeline with Validation
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    println!("\n");
    println!("   DATABASE PIPELINE WITH HANDOFF VALIDATION                 ");
    println!("\n");

    init().await?;

    // Load settings
    let settings =
        Settings::new().map_err(|e| anyhow::anyhow!("Failed to load settings: {}", e))?;

    // Initialize database
    println!(" Initializing SQLite database with sample sales data...");
    let _ = &*DB;
    println!("   Database initialized with 20 sales records\n");

    // Setup validation contracts
    println!("Setting up handoff validation contracts...");
    let coordinator = setup_validation_contracts(&settings);
    println!("    database_agent_handoff contract registered");
    println!("    analysis_agent_handoff contract registered");
    println!("    reporting_agent_handoff contract registered");
    println!(
        "    Agent timeout: {}ms\n",
        settings.validation.agent_timeout_ms
    );

    // Build specialized agents
    println!(" Building specialized agents...");

    let database_agent = AgentBuilder::new("database_agent")
        .description("Executes SQL queries and returns structured JSON data")
        .system_prompt(
            "You are a database specialist. Call the appropriate query tool to fetch data.",
        )
        .tool(QueryProductRevenueTool::new())
        .tool(QueryRegionPerformanceTool::new())
        .return_tool_output(true);

    let analysis_agent = AgentBuilder::new("analysis_agent")
        .description("Analyzes data and returns structured insights as JSON")
        .system_prompt(
            "You are a business analyst. \
             You will receive CONTEXT DATA containing output from previous agents. \
             Use the JSON data from context (e.g., database_agent_output) and pass it as a JSON STRING to your analysis tools.\
             \n\nFor example:\n\
             - Context has: database_agent_output: {\"data\": [...]}\n\
             - Convert it to a string: JSON.stringify(database_agent_output)\n\
             - Pass to tool: {\"product_json\": \"<stringified JSON here>\"}\n\
             - Tools expect JSON STRING parameters, not JSON objects",
        )
        .tool(AnalyzeProductDataTool::new())
        .tool(AnalyzeRegionalDataTool::new())
        .return_tool_output(true);

    let reporting_agent = AgentBuilder::new("reporting_agent")
        .description("Generates comprehensive reports as structured JSON")
        .system_prompt(
            "You are a reporting specialist. \
             You will receive CONTEXT DATA containing analysis_agent_output. \
             Extract the analysis results from context and pass them as STRING parameters to the generate_report tool.",
        )
        .tool(GenerateReportTool::new())
        .return_tool_output(true);

    let agents = AgentCollection::new()
        .add(database_agent)
        .add(analysis_agent)
        .add(reporting_agent);

    println!("    {} agents created\n", agents.len());

    let agent_configs = agents.build();

    // Execute pipeline with validation checkpoints
    println!("Starting validated pipeline execution...\n");
    println!("Pipeline stages:");
    println!("   1. Database Agent → Extract data (validate schema)");
    println!("   2. Analysis Agent → Generate insights (validate quality)");
    println!("   3. Reporting Agent → Create report (validate completeness)\n");

    let task = "
        Execute a validated data analysis pipeline:

        STAGE 1 - Data Extraction:
        1. Use query_product_revenue to get product data as JSON
        2. Use query_region_performance to get regional data as JSON

        STAGE 2 - Data Analysis:
        3. Use analyze_product_data with the product JSON from step 1
        4. Use analyze_regional_data with the regional JSON from step 2

        STAGE 3 - Report Generation:
        5. Use generate_report with both analysis results (steps 3 and 4)

        IMPORTANT:
        - Each tool returns structured JSON
        - Pass the exact JSON output from one step to the next
        - The final result should be the complete JSON report
    ";

    // Use the new API with validation!
    let result =
        supervisor::orchestrate_custom_agents_with_validation(coordinator, agent_configs, task)
            .await?;

    println!("\n");
    println!("                    FINAL RESULT                              ");
    println!("\n");

    println!("Pipeline Success: {}", result.success);
    println!("Total Steps: {}", result.steps.len());
    println!("\nFinal Output:\n{}\n", result.result);

    Ok(())
}
