//! Agent with Custom Macro-Defined Tools
//!
//! Demonstrates creating domain-specific tools using the #[tool_fn] macro
//! and using them with the LLM agent (similar to supervisor_usage.rs).
//!
//! This example shows:
//! - Creating custom tools with #[tool_fn] macro (Python/MCP familiar)
//! - Using custom tools with the LLM agent
//! - LLM autonomously selecting and executing custom tools to complete tasks

#![allow(unused_variables)]

use anyhow::Result;
use actorus::tool_fn;
use actorus::tools::Tool;
use actorus::{init, agent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

// ============================================================================
// Custom Tools - Data Management
// ============================================================================

/// Add an item to the inventory
#[tool_fn(
    name = "add_item",
    description = "Add a new item to the inventory with name and quantity"
)]
async fn add_item(name: String, quantity: i64, category: Option<String>) -> Result<String> {
    let category_str = category
        .as_ref()
        .map(|c| format!(" in category '{}'", c))
        .unwrap_or_default();

    Ok(format!(
        "Added {} units of '{}'{} to inventory",
        quantity, name, category_str
    ))
}

/// Search for items in inventory
#[tool_fn(
    name = "search_items",
    description = "Search for items in the inventory by category"
)]
async fn search_items(category: String) -> Result<String> {
    Ok(format!(
        "Found 3 items in category '{}': Widget, Gadget, Device",
        category
    ))
}

/// Get inventory count
#[tool_fn(
    name = "count_items",
    description = "Count total number of items in the inventory"
)]
async fn count_items() -> Result<String> {
    Ok("Total items in inventory: 15".to_string())
}

// ============================================================================
// Custom Tools - Text Processing
// ============================================================================

/// Transform text
#[tool_fn(
    name = "transform_text",
    description = "Transform text using operations: uppercase, lowercase, reverse"
)]
async fn transform_text(text: String, operation: String) -> Result<String> {
    match operation.as_str() {
        "uppercase" => Ok(text.to_uppercase()),
        "lowercase" => Ok(text.to_lowercase()),
        "reverse" => Ok(text.chars().rev().collect()),
        _ => Err(anyhow::anyhow!(
            "Unknown operation. Use: uppercase, lowercase, reverse"
        )),
    }
}

// ============================================================================
// Custom Tools - Calculator
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum MathOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Calculate with enums
#[tool_fn(
    name = "calculate",
    description = "Perform math operations: add, subtract, multiply, divide"
)]
async fn calculate(op: MathOp, a: i64, b: i64) -> Result<String> {
    let result = match op {
        MathOp::Add => a + b,
        MathOp::Subtract => a - b,
        MathOp::Multiply => a * b,
        MathOp::Divide => {
            if b == 0 {
                return Err(anyhow::anyhow!("Cannot divide by zero"));
            }
            a / b
        }
    };
    Ok(format!("{:?}: {} and {} = {}", op, a, b, result))
}

// ============================================================================
// Main - LLM Agent with Custom Tools
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    println!("\n=== Agent with Custom Macro-Defined Tools ===\n");

    init().await?;

    // Create custom tools collection
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(AddItemTool::new()),
        Arc::new(SearchItemsTool::new()),
        Arc::new(CountItemsTool::new()),
        Arc::new(TransformTextTool::new()),
        Arc::new(CalculateTool::new()),
    ];

    println!("Registered {} custom tools:", tools.len());
    for tool in &tools {
        println!("  - {}: {}", tool.metadata().name, tool.metadata().description);
    }
    println!();

    // ========================================================================
    // Task 1: Inventory Management
    // ========================================================================
    println!("Task 1: Manage Inventory");
    println!("------------------------");
    let result = agent::run_task_with_tools(
        tools.clone(),
        "Add 100 units of 'Premium Widget' in the Electronics category to inventory"
    ).await?;

    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);
    println!("Steps taken: {}", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        println!("  Step {}: {}", i + 1, step.thought);
        if let Some(action) = &step.action {
            println!("    Action: {}", action);
        }
    }
    println!();

    // ========================================================================
    // Task 2: Search and Count
    // ========================================================================
    println!("Task 2: Search and Count");
    println!("-----------------------");
    let result = agent::run_task_with_tools(
        tools.clone(),
        "Search for items in the Electronics category and then count the total inventory"
    ).await?;

    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);
    println!("Steps taken: {}", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        println!("  Step {}: {}", i + 1, step.thought);
        if let Some(action) = &step.action {
            println!("    Action: {}", action);
        }
    }
    println!();

    // ========================================================================
    // Task 3: Text Transformation
    // ========================================================================
    println!("Task 3: Transform Text");
    println!("---------------------");
    let result = agent::run_task_with_tools(
        tools.clone(),
        "Transform the text 'hello world' to uppercase"
    ).await?;

    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);
    println!("Steps taken: {}", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        println!("  Step {}: {}", i + 1, step.thought);
        if let Some(action) = &step.action {
            println!("    Action: {}", action);
        }
    }
    println!();

    // ========================================================================
    // Task 4: Math Calculation
    // ========================================================================
    println!("Task 4: Calculate");
    println!("----------------");
    let result = agent::run_task_with_tools(
        tools.clone(),
        "Calculate 25 multiplied by 4"
    ).await?;

    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);
    println!("Steps taken: {}", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        println!("  Step {}: {}", i + 1, step.thought);
        if let Some(action) = &step.action {
            println!("    Action: {}", action);
        }
    }
    println!();

    println!("=== Agent with Custom Tools Example Complete ===\n");

    Ok(())
}
