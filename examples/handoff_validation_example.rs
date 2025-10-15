//! Handoff Protocol Validation Example
//!
//! Demonstrates structured handoff validation between agents with:
//! - Schema-based validation
//! - Metadata enrichment
//! - Execution time limits

use actorus::actors::handoff::{
    enrich_metadata_with_validation, HandoffContract, HandoffCoordinator,
};
use actorus::actors::messages::{
    AgentResponse, AgentStep, CompletionStatus, OutputMetadata, OutputSchema, ToolCallMetadata,
    ValidationResult, ValidationRule, ValidationType,
};
use std::collections::HashMap;

fn main() {
    println!("\n=== Handoff Protocol Validation Example ===\n");

    // Create a handoff coordinator
    let mut coordinator = HandoffCoordinator::new();

    // Register contracts for agent handoffs
    register_example_contracts(&mut coordinator);

    // Example 1: Successful handoff with valid data
    println!("Example 1: Valid Database Query Handoff");
    println!("----------------------------------------");
    let db_response = create_database_response();
    let validation = coordinator.validate_handoff("database_to_analysis", &db_response);
    print_validation_result(&validation);

    // Example 2: Analysis handoff with enriched metadata
    println!("\nExample 2: Analysis Handoff with Metadata Enrichment");
    println!("-----------------------------------------------------");
    let analysis_response = create_analysis_response();
    let validation = coordinator.validate_handoff("analysis_to_reporting", &analysis_response);

    // Enrich metadata with validation results
    let enriched_metadata = enrich_metadata_with_validation(
        match &analysis_response {
            AgentResponse::Success { metadata, .. } => metadata.clone(),
            _ => None,
        },
        validation.clone(),
        "1.0".to_string(),
    );

    println!(
        "Validation Result: {}",
        if validation.valid { "VALID" } else { "INVALID" }
    );
    println!("Enriched Metadata:");
    println!("  - Confidence: {:.2}", enriched_metadata.confidence);
    println!(
        "  - Execution Time: {}ms",
        enriched_metadata.execution_time_ms
    );
    println!(
        "  - Schema Version: {}",
        enriched_metadata.schema_version.unwrap_or_default()
    );
    println!("  - Tool Calls: {}", enriched_metadata.tool_calls.len());

    for tool_call in &enriched_metadata.tool_calls {
        println!(
            "    - {} ({} -> {} bytes in {}ms)",
            tool_call.tool_name, tool_call.input_size, tool_call.output_size, tool_call.duration_ms
        );
    }

    // Example 3: Using built-in contract templates
    println!("\nExample 3: Built-in Contract Templates");
    println!("---------------------------------------");

    let db_contract = HandoffCoordinator::database_output_contract();
    println!("Database Output Contract:");
    println!("  - From: {}", db_contract.from_agent);
    println!("  - To: {}", db_contract.to_agent.unwrap_or_default());
    println!(
        "  - Max Execution Time: {}ms",
        db_contract.max_execution_time_ms.unwrap_or(0)
    );

    let analysis_contract = HandoffCoordinator::analysis_output_contract();
    println!("\nAnalysis Output Contract:");
    println!("  - From: {}", analysis_contract.from_agent);
    println!("  - To: {}", analysis_contract.to_agent.unwrap_or_default());
    println!(
        "  - Max Execution Time: {}ms",
        analysis_contract.max_execution_time_ms.unwrap_or(0)
    );

    println!("\n=== Handoff Protocol Example Complete ===\n");
}

fn register_example_contracts(coordinator: &mut HandoffCoordinator) {
    // Contract for database -> analysis handoff
    let mut db_field_types = HashMap::new();
    db_field_types.insert("query_result".to_string(), "string".to_string());
    db_field_types.insert("row_count".to_string(), "number".to_string());

    coordinator.register_contract(
        "database_to_analysis".to_string(),
        HandoffContract {
            from_agent: "database_agent".to_string(),
            to_agent: Some("analysis_agent".to_string()),
            schema: OutputSchema {
                schema_version: "1.0".to_string(),
                required_fields: vec!["query_result".to_string()],
                optional_fields: vec!["row_count".to_string()],
                field_types: db_field_types,
                validation_rules: vec![ValidationRule {
                    field: "query_result".to_string(),
                    rule_type: ValidationType::MinLength,
                    constraint: "10".to_string(),
                }],
            },
            max_execution_time_ms: Some(5000),
        },
    );

    // Contract for analysis -> reporting handoff
    let mut analysis_field_types = HashMap::new();
    analysis_field_types.insert("summary".to_string(), "string".to_string());
    analysis_field_types.insert("metrics".to_string(), "object".to_string());

    coordinator.register_contract(
        "analysis_to_reporting".to_string(),
        HandoffContract {
            from_agent: "analysis_agent".to_string(),
            to_agent: Some("reporting_agent".to_string()),
            schema: OutputSchema {
                schema_version: "1.0".to_string(),
                required_fields: vec!["summary".to_string()],
                optional_fields: vec!["metrics".to_string()],
                field_types: analysis_field_types,
                validation_rules: vec![ValidationRule {
                    field: "summary".to_string(),
                    rule_type: ValidationType::MinLength,
                    constraint: "50".to_string(),
                }],
            },
            max_execution_time_ms: Some(10000),
        },
    );
}

fn create_database_response() -> AgentResponse {
    AgentResponse::Success {
        result: r#"{"query_result": "SELECT * FROM sales returned 100 rows", "row_count": 100}"#
            .to_string(),
        steps: vec![AgentStep {
            iteration: 0,
            thought: "Executing SQL query".to_string(),
            action: Some("execute_query".to_string()),
            observation: Some("Query successful".to_string()),
        }],
        metadata: Some(OutputMetadata {
            confidence: 0.95,
            execution_time_ms: 250,
            agent_name: Some("database_agent".to_string()),
            tool_calls: vec![ToolCallMetadata {
                tool_name: "execute_query".to_string(),
                input_size: 45,
                output_size: 120,
                duration_ms: 200,
                success: true,
            }],
            ..Default::default()
        }),
        completion_status: Some(CompletionStatus::Complete { confidence: 0.95 }),
    }
}

fn create_analysis_response() -> AgentResponse {
    AgentResponse::Success {
        result: r#"{"summary": "Sales data shows strong performance in Q4 with 25% growth over previous quarter. Key drivers include product launches and seasonal trends.", "metrics": {"total_sales": 1000000, "growth_rate": 0.25}}"#.to_string(),
        steps: vec![
            AgentStep {
                iteration: 0,
                thought: "Analyzing sales trends".to_string(),
                action: Some("analyze_trends".to_string()),
                observation: Some("Trends analyzed".to_string()),
            },
            AgentStep {
                iteration: 1,
                thought: "Calculating metrics".to_string(),
                action: Some("calculate_metrics".to_string()),
                observation: Some("Metrics calculated".to_string()),
            },
        ],
        metadata: Some(OutputMetadata {
            confidence: 0.88,
            execution_time_ms: 1500,
            agent_name: Some("analysis_agent".to_string()),
            tool_calls: vec![
                ToolCallMetadata {
                    tool_name: "analyze_trends".to_string(),
                    input_size: 120,
                    output_size: 350,
                    duration_ms: 800,
                    success: true,
                },
                ToolCallMetadata {
                    tool_name: "calculate_metrics".to_string(),
                    input_size: 80,
                    output_size: 200,
                    duration_ms: 600,
                    success: true,
                },
            ],
            ..Default::default()
        }),
        completion_status: Some(CompletionStatus::Complete { confidence: 0.88 }),
    }
}

fn print_validation_result(validation: &ValidationResult) {
    if validation.valid {
        println!(" Validation PASSED");
    } else {
        println!(" Validation FAILED");
        println!("  Errors:");
        for error in &validation.errors {
            println!("    - {}: {}", error.field, error.message);
        }
    }

    if !validation.warnings.is_empty() {
        println!("  Warnings:");
        for warning in &validation.warnings {
            println!("    - {}", warning);
        }
    }
}
