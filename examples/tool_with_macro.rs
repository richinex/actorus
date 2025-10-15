//! Example: Creating a Tool with Macros
//!
//! This demonstrates how the tool_metadata! macro simplifies tool creation

use anyhow::Result;
use async_trait::async_trait;
use actorus::tools::{Tool, ToolMetadata, ToolResult};
use actorus::{tool_metadata, tool_result, validate_optional_string, validate_required_string};
use serde_json::Value;

/// Example tool using macros for cleaner code
pub struct GreetTool;

#[async_trait]
impl Tool for GreetTool {
    fn metadata(&self) -> ToolMetadata {
        // BEFORE: Manual definition (20+ lines)
        // AFTER: Clean declarative syntax (8 lines)
        tool_metadata! {
            name: "greet",
            description: "Greet a person with a custom message",
            parameters: [
                {
                    name: "name",
                    type: "string",
                    description: "The person's name to greet",
                    required: true
                },
                {
                    name: "greeting",
                    type: "string",
                    description: "Custom greeting (optional, defaults to 'Hello')",
                    required: false
                }
            ]
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        // BEFORE: Manual string extraction and validation
        // AFTER: Simple macro
        let _name = validate_required_string!(args, "name");

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        // Use the validation macros for clean parameter extraction
        let name = validate_required_string!(args, "name");
        let greeting = validate_optional_string!(args, "greeting", "Hello");

        let message = format!("{}, {}!", greeting, name);

        // BEFORE: Ok(ToolResult::success(...))
        // AFTER: Clean macro
        tool_result!(success: message)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    use serde_json::json;

    println!("=== Tool Macro Example ===\n");

    let tool = GreetTool;

    // Show metadata
    let metadata = tool.metadata();
    println!("Tool: {}", metadata.name);
    println!("Description: {}", metadata.description);
    println!("\nParameters:");
    for param in &metadata.parameters {
        let req = if param.required {
            "required"
        } else {
            "optional"
        };
        println!(
            "  - {} ({}): {} [{}]",
            param.name, param.param_type, param.description, req
        );
    }

    // Test execution
    println!("\n--- Test 1: With default greeting ---");
    let result = tool
        .execute(json!({
            "name": "Alice"
        }))
        .await?;
    println!("Result: {}", result.output);

    println!("\n--- Test 2: With custom greeting ---");
    let result = tool
        .execute(json!({
            "name": "Bob",
            "greeting": "Hi there"
        }))
        .await?;
    println!("Result: {}", result.output);

    println!("\n--- Test 3: Missing required parameter ---");
    match tool.execute(json!({})).await {
        Ok(result) => {
            if !result.success {
                println!("Error (expected): {}", result.error.unwrap());
            }
        }
        Err(e) => {
            println!("Error (expected): {}", e);
        }
    }

    println!("\n=== Example Complete ===");

    Ok(())
}
