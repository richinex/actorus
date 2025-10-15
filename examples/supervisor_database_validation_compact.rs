//! Compact Database Pipeline with Handoff Validation
//!
//! Demonstrates quality gates between agents:
//! Database Agent → Validation → Analysis Agent → Validation → Report
//!
//! Key validation features:
//! - Schema compliance (required fields, types)
//! - Data quality rules (ranges, enums)
//! - Execution time limits
//! - Structured JSON handoffs

use actorus::actors::handoff::{HandoffContract, HandoffCoordinator};
use actorus::actors::messages::{OutputSchema, ValidationRule, ValidationType};
use actorus::tool_fn;
use actorus::{init, supervisor, AgentBuilder, AgentCollection, Settings};
use anyhow::Result;
use once_cell::sync::Lazy;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;

// ============================================================================
// Database Setup
// ============================================================================

static DB: Lazy<Mutex<Connection>> = Lazy::new(|| {
    let conn = Connection::open_in_memory().expect("Failed to create database");
    conn.execute(
        "CREATE TABLE sales (product TEXT, region TEXT, quantity INTEGER, price REAL)",
        [],
    )
    .unwrap();

    let data = vec![
        ("Laptop", "North", 45, 1299.99),
        ("Laptop", "South", 32, 1299.99),
        ("Phone", "North", 120, 899.99),
        ("Phone", "South", 95, 899.99),
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
// Database Agent Tools (returns structured JSON)
// ============================================================================

#[tool_fn(name = "query_revenue", description = "Query revenue (JSON)")]
async fn query_revenue() -> Result<String> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT product, SUM(quantity * price) as revenue
         FROM sales GROUP BY product ORDER BY revenue DESC",
    )?;

    #[derive(Serialize)]
    struct ProductData {
        product: String,
        revenue: f64,
    }

    #[derive(Serialize)]
    struct QueryResult {
        data: Vec<ProductData>,
        row_count: usize,
        status: String,
    }

    let mut products = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    for row in rows {
        let (product, revenue) = row?;
        products.push(ProductData { product, revenue });
    }

    let result = QueryResult {
        row_count: products.len(),
        data: products,
        status: "success".to_string(),
    };

    Ok(serde_json::to_string_pretty(&result)?)
}

// ============================================================================
// Analysis Agent Tools (returns structured JSON)
// ============================================================================

#[tool_fn(name = "analyze_data", description = "Analyze sales data (JSON)")]
async fn analyze_data(_product_json: String) -> Result<String> {
    #[derive(Serialize)]
    struct AnalysisResult {
        insights: Vec<String>,
        metrics: HashMap<String, f64>,
        confidence_score: f64,
    }

    let mut metrics = HashMap::new();
    metrics.insert("avg_revenue".to_string(), 50000.0);
    metrics.insert("product_count".to_string(), 2.0);

    let result = AnalysisResult {
        insights: vec![
            "Laptop leads revenue generation".to_string(),
            "Phone shows strong volume".to_string(),
        ],
        metrics,
        confidence_score: 0.92,
    };

    Ok(serde_json::to_string_pretty(&result)?)
}

// ============================================================================
// Reporting Agent Tools (returns structured JSON)
// ============================================================================

#[tool_fn(name = "generate_report", description = "Generate report (JSON)")]
async fn generate_report(_analysis: String) -> Result<String> {
    #[derive(Serialize)]
    struct Report {
        title: String,
        summary: String,
        key_findings: Vec<String>,
        confidence: f64,
    }

    let report = Report {
        title: "Sales Analysis Report".to_string(),
        summary: "Strong performance with clear growth opportunities in regional expansion."
            .to_string(),
        key_findings: vec![
            "Laptop revenue: $58,499.68".to_string(),
            "Phone revenue: $107,999.05".to_string(),
            "North region leads performance".to_string(),
        ],
        confidence: 0.90,
    };

    Ok(serde_json::to_string_pretty(&report)?)
}

// ============================================================================
// Validation Contracts Setup
// ============================================================================

fn setup_validation(settings: &Settings) -> HandoffCoordinator {
    let mut coordinator = HandoffCoordinator::new();

    // Contract 1: Database → Analysis
    let mut db_types = HashMap::new();
    db_types.insert("data".to_string(), "array".to_string());
    db_types.insert("status".to_string(), "string".to_string());

    coordinator.register_contract(
        "database_agent_handoff".to_string(),
        HandoffContract {
            from_agent: "database_agent".to_string(),
            to_agent: Some("analysis_agent".to_string()),
            schema: OutputSchema {
                schema_version: "1.0".to_string(),
                required_fields: vec!["data".to_string(), "status".to_string()],
                optional_fields: vec!["row_count".to_string()],
                field_types: db_types,
                validation_rules: vec![
                    ValidationRule {
                        field: "status".to_string(),
                        rule_type: ValidationType::Enum,
                        constraint: "success,partial,failed".to_string(),
                    },
                    ValidationRule {
                        field: "row_count".to_string(),
                        rule_type: ValidationType::Range,
                        constraint: "1..100".to_string(),
                    },
                ],
            },
            max_execution_time_ms: Some(settings.validation.agent_timeout_ms),
        },
    );

    // Contract 2: Analysis → Reporting
    let mut analysis_types = HashMap::new();
    analysis_types.insert("insights".to_string(), "array".to_string());
    analysis_types.insert("confidence_score".to_string(), "number".to_string());

    coordinator.register_contract(
        "analysis_agent_handoff".to_string(),
        HandoffContract {
            from_agent: "analysis_agent".to_string(),
            to_agent: Some("reporting_agent".to_string()),
            schema: OutputSchema {
                schema_version: "1.0".to_string(),
                required_fields: vec!["insights".to_string(), "confidence_score".to_string()],
                optional_fields: vec!["metrics".to_string()],
                field_types: analysis_types,
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

    // Contract 3: Reporting → Final
    let mut report_types = HashMap::new();
    report_types.insert("title".to_string(), "string".to_string());
    report_types.insert("key_findings".to_string(), "array".to_string());

    coordinator.register_contract(
        "reporting_agent_handoff".to_string(),
        HandoffContract {
            from_agent: "reporting_agent".to_string(),
            to_agent: None,
            schema: OutputSchema {
                schema_version: "1.0".to_string(),
                required_fields: vec!["title".to_string(), "summary".to_string()],
                optional_fields: vec!["key_findings".to_string()],
                field_types: report_types,
                validation_rules: vec![ValidationRule {
                    field: "summary".to_string(),
                    rule_type: ValidationType::MinLength,
                    constraint: "20".to_string(),
                }],
            },
            max_execution_time_ms: Some(settings.validation.agent_timeout_ms),
        },
    );

    coordinator
}

// ============================================================================
// Main Pipeline
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("\n");
    println!("   DATABASE PIPELINE WITH VALIDATION          ");
    println!("\n");

    init().await?;

    let settings = Settings::new()?;

    // Initialize database
    let _ = &*DB;
    println!("Database initialized\n");

    // Setup validation
    println!("Setting up validation contracts:");
    let coordinator = setup_validation(&settings);
    println!("    database_agent_handoff");
    println!("    analysis_agent_handoff");
    println!("    reporting_agent_handoff");
    println!("    Timeout: {}ms\n", settings.validation.agent_timeout_ms);

    // Build agents with validation
    let database_agent = AgentBuilder::new("database_agent")
        .description("Executes SQL queries")
        .system_prompt("You are a database specialist. Call query tools to fetch JSON data.")
        .tool(QueryRevenueTool::new())
        .return_tool_output(true);

    let analysis_agent = AgentBuilder::new("analysis_agent")
        .description("Analyzes data")
        .system_prompt(
            "You are an analyst. \
             Use the database_agent_output from context and pass it to analysis tools as a JSON string.",
        )
        .tool(AnalyzeDataTool::new())
        .return_tool_output(true);

    let reporting_agent = AgentBuilder::new("reporting_agent")
        .description("Generates reports")
        .system_prompt(
            "You are a reporter. \
             Use analysis_agent_output from context to generate reports.",
        )
        .tool(GenerateReportTool::new())
        .return_tool_output(true);

    let agents = AgentCollection::new()
        .add(database_agent)
        .add(analysis_agent)
        .add(reporting_agent);

    println!("Created {} agents\n", agents.len());

    let agent_configs = agents.build();

    // Execute with validation
    println!("Executing validated pipeline:");
    println!("   1. Extract data (validate schema)");
    println!("   2. Analyze data (validate quality)");
    println!("   3. Generate report (validate completeness)\n");

    let task = "
        Execute validated pipeline:
        1. Use query_revenue to get product data as JSON
        2. Use analyze_data with the product JSON
        3. Use generate_report with analysis results

        Each step returns structured JSON that is validated before handoff.
    ";

    let result =
        supervisor::orchestrate_custom_agents_with_validation(coordinator, agent_configs, task)
            .await?;

    println!("\n");
    println!("            VALIDATION RESULTS                ");
    println!("\n");

    println!("Success: {}", result.success);
    println!("Steps: {}", result.steps.len());
    println!("\nFinal Output:\n{}\n", result.result);

    Ok(())
}
