//! Supervisor Orchestrating Custom Specialized Agents
//!
//! Demonstrates the supervisor pattern with custom macro-defined tools.
//! Similar to supervisor_usage.rs but with domain-specific agents instead of built-in ones.
//!
//! This example shows:
//! - Creating custom tools with #[tool_fn] macro
//! - Building specialized agents with custom tools
//! - Using supervisor to orchestrate multiple custom agents
//! - LLM autonomously decomposing complex tasks across agents

#![allow(unused_variables)]

use actorus::tool_fn;
use actorus::{init, supervisor, AgentBuilder, AgentCollection};
use anyhow::Result;
use serde::{Deserialize, Serialize};
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

/// Analyze text
#[tool_fn(
    name = "analyze_text",
    description = "Analyze text and return statistics"
)]
async fn analyze_text(text: String) -> Result<String> {
    let word_count = text.split_whitespace().count();
    let char_count = text.chars().count();
    let has_uppercase = text.chars().any(|c| c.is_uppercase());

    Ok(format!(
        "Text analysis: {} words, {} characters, contains uppercase: {}",
        word_count, char_count, has_uppercase
    ))
}

// ============================================================================
// Custom Tools - Math Calculator
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

/// Calculate percentage
#[tool_fn(name = "percentage", description = "Calculate percentage of a number")]
async fn percentage(value: i64, percent: i64) -> Result<String> {
    let result = (value * percent) / 100;
    Ok(format!("{}% of {} = {}", percent, value, result))
}

// ============================================================================
// Main - Supervisor Orchestrating Custom Agents
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    println!("\n=== Supervisor Orchestrating Custom Specialized Agents ===\n");

    init().await?;

    // ========================================================================
    // Build Custom Specialized Agents - Using AgentBuilder
    // ========================================================================

    // Data Management Agent
    let data_agent = AgentBuilder::new("data_agent")
        .description("Manages inventory data - can add items, search by category, and count totals")
        .system_prompt("You are a data management specialist. Use your inventory tools to manage and query data.")
        .tool(AddItemTool::new())
        .tool(SearchItemsTool::new())
        .tool(CountItemsTool::new());

    // Text Processing Agent
    let text_agent = AgentBuilder::new("text_agent")
        .description("Processes and analyzes text - can transform case, reverse text, and analyze statistics")
        .system_prompt("You are a text processing specialist. Use your tools to transform and analyze text.")
        .tool(TransformTextTool::new())
        .tool(AnalyzeTextTool::new());

    // Math Calculator Agent
    let math_agent = AgentBuilder::new("math_agent")
        .description("Performs mathematical calculations - can add, subtract, multiply, divide, and calculate percentages")
        .system_prompt("You are a math specialist. Use your calculation tools to solve mathematical problems.")
        .tool(CalculateTool::new())
        .tool(PercentageTool::new());

    // Collect agents
    let agents = AgentCollection::new()
        .add(data_agent)
        .add(text_agent)
        .add(math_agent);

    println!("Created {} custom specialized agents:", agents.len());
    for (name, description) in agents.list_agents() {
        println!("  - {}: {}", name, description);
    }

    // Build agent configurations
    let agent_configs = agents.build();
    println!();

    // ========================================================================
    // Task 1: Multi-agent Coordination - Data + Math
    // ========================================================================
    println!("Task 1: Complex Multi-Agent Task (Data + Math)");
    println!("==============================================");
    let result = supervisor::orchestrate_custom_agents(
        agent_configs.clone(),
        "Add 100 units of 'Premium Widget' to inventory in Electronics category, \
         then count the total inventory, and finally calculate what 25% of that total would be",
    )
    .await?;

    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);
    println!("Orchestration steps: {}", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        println!("  Step {}: {}", i + 1, step.thought);
        if let Some(action) = &step.action {
            println!("    Action: {}", action);
        }
    }
    println!();

    // ========================================================================
    // Task 2: Multi-agent Coordination - Text + Data
    // ========================================================================
    println!("Task 2: Complex Multi-Agent Task (Text + Data)");
    println!("==============================================");
    let result = supervisor::orchestrate_custom_agents(
        agent_configs.clone(),
        "Transform the text 'hello world' to uppercase, analyze the transformed text, \
         then search for items in the Electronics category",
    )
    .await?;

    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);
    println!("Orchestration steps: {}", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        println!("  Step {}: {}", i + 1, step.thought);
        if let Some(action) = &step.action {
            println!("    Action: {}", action);
        }
    }
    println!();

    // ========================================================================
    // Task 3: All Three Agents - Data + Text + Math
    // ========================================================================
    println!("Task 3: Complex Multi-Agent Task (All Agents)");
    println!("=============================================");
    let result = supervisor::orchestrate_custom_agents(
        agent_configs.clone(),
        "First, count the total inventory items. Second, calculate 50% of that count. \
         Third, create a text summary by transforming 'inventory report' to uppercase, \
         then analyze that summary text",
    )
    .await?;

    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);
    println!("Orchestration steps: {}", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        println!("  Step {}: {}", i + 1, step.thought);
        if let Some(action) = &step.action {
            println!("    Action: {}", action);
        }
    }
    println!();

    Ok(())
}
