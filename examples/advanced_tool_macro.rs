//! Advanced Tool Macro Example
//!
//! This demonstrates edge cases and advanced usage of tool macros:
//! - Multiple parameter types (strings, numbers)
//! - Mix of required and optional parameters
//! - Number validation
//! - Custom business logic validation
//! - Error handling patterns

use actorus::tools::{Tool, ToolMetadata, ToolResult};
use actorus::{
    tool_metadata, tool_result, validate_optional_string, validate_required_number,
    validate_required_string,
};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Calculator tool demonstrating number validation
pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn metadata(&self) -> ToolMetadata {
        tool_metadata! {
            name: "calculate",
            description: "Perform basic arithmetic operations on two numbers",
            parameters: [
                {
                    name: "operation",
                    type: "string",
                    description: "Operation to perform: add, subtract, multiply, divide",
                    required: true
                },
                {
                    name: "a",
                    type: "number",
                    description: "First number",
                    required: true
                },
                {
                    name: "b",
                    type: "number",
                    description: "Second number",
                    required: true
                }
            ]
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let operation = validate_required_string!(args, "operation");
        let _a = validate_required_number!(args, "a");
        let b = validate_required_number!(args, "b");

        // Custom validation: check valid operations
        if !["add", "subtract", "multiply", "divide"].contains(&operation) {
            return Err(anyhow::anyhow!(
                "Invalid operation '{}'. Must be: add, subtract, multiply, divide",
                operation
            ));
        }

        // Custom validation: prevent division by zero
        if operation == "divide" && b == 0 {
            return Err(anyhow::anyhow!("Cannot divide by zero"));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let operation = validate_required_string!(args, "operation");
        let a = validate_required_number!(args, "a");
        let b = validate_required_number!(args, "b");

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => a / b,
            _ => unreachable!(), // Already validated
        };

        tool_result!(success: format!("{} {} {} = {}", a, operation, b, result))
    }
}

/// File search tool demonstrating optional parameters
pub struct FileSearchTool;

#[async_trait]
impl Tool for FileSearchTool {
    fn metadata(&self) -> ToolMetadata {
        tool_metadata! {
            name: "search_files",
            description: "Search for files by name pattern with optional filters",
            parameters: [
                {
                    name: "pattern",
                    type: "string",
                    description: "Search pattern (e.g., '*.rs', 'test*')",
                    required: true
                },
                {
                    name: "directory",
                    type: "string",
                    description: "Directory to search in (defaults to current directory)",
                    required: false
                },
                {
                    name: "max_results",
                    type: "number",
                    description: "Maximum number of results to return (defaults to 10)",
                    required: false
                }
            ]
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let _pattern = validate_required_string!(args, "pattern");

        // Optional number validation
        if let Some(max) = args.get("max_results").and_then(|v| v.as_i64()) {
            if max <= 0 {
                return Err(anyhow::anyhow!("max_results must be positive"));
            }
            if max > 1000 {
                return Err(anyhow::anyhow!("max_results cannot exceed 1000"));
            }
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let pattern = validate_required_string!(args, "pattern");
        let directory = validate_optional_string!(args, "directory", ".");
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_i64())
            .unwrap_or(10);

        // Simulate file search
        let message = format!(
            "Searching for '{}' in '{}' (max {} results)",
            pattern, directory, max_results
        );

        tool_result!(success: message)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    use serde_json::json;

    println!("=== Advanced Tool Macro Example ===\n");

    // Test CalculatorTool
    println!("--- Calculator Tool Tests ---\n");
    let calc = CalculatorTool;

    println!("Test 1: Addition");
    let result = calc
        .execute(json!({
            "operation": "add",
            "a": 5,
            "b": 3
        }))
        .await?;
    println!(" {}\n", result.output);

    println!("Test 2: Division");
    let result = calc
        .execute(json!({
            "operation": "divide",
            "a": 10,
            "b": 2
        }))
        .await?;
    println!(" {}\n", result.output);

    println!("Test 3: Invalid operation (should fail)");
    match calc
        .execute(json!({
            "operation": "power",
            "a": 2,
            "b": 3
        }))
        .await
    {
        Ok(result) if !result.success => {
            println!(" Validation caught: {}\n", result.error.unwrap());
        }
        Err(e) => {
            println!(" Validation caught: {}\n", e);
        }
        _ => println!(" Should have failed\n"),
    }

    println!("Test 4: Division by zero (should fail)");
    match calc
        .execute(json!({
            "operation": "divide",
            "a": 10,
            "b": 0
        }))
        .await
    {
        Ok(result) if !result.success => {
            println!(" Validation caught: {}\n", result.error.unwrap());
        }
        Err(e) => {
            println!(" Validation caught: {}\n", e);
        }
        _ => println!(" Should have failed\n"),
    }

    println!("Test 5: Missing required number parameter (should fail)");
    match calc
        .execute(json!({
            "operation": "add",
            "a": 5
        }))
        .await
    {
        Ok(result) if !result.success => {
            println!(" Validation caught: {}\n", result.error.unwrap());
        }
        Err(e) => {
            println!(" Validation caught: {}\n", e);
        }
        _ => println!(" Should have failed\n"),
    }

    // Test FileSearchTool
    println!("\n--- File Search Tool Tests ---\n");
    let search = FileSearchTool;

    println!("Test 6: Basic search with defaults");
    let result = search
        .execute(json!({
            "pattern": "*.rs"
        }))
        .await?;
    println!(" {}\n", result.output);

    println!("Test 7: Search with custom directory");
    let result = search
        .execute(json!({
            "pattern": "test*",
            "directory": "/tmp"
        }))
        .await?;
    println!(" {}\n", result.output);

    println!("Test 8: Search with max_results");
    let result = search
        .execute(json!({
            "pattern": "*.txt",
            "directory": "~/Documents",
            "max_results": 5
        }))
        .await?;
    println!(" {}\n", result.output);

    println!("Test 9: Invalid max_results (should fail)");
    match search
        .execute(json!({
            "pattern": "*.txt",
            "max_results": -1
        }))
        .await
    {
        Ok(result) if !result.success => {
            println!(" Validation caught: {}\n", result.error.unwrap());
        }
        Err(e) => {
            println!(" Validation caught: {}\n", e);
        }
        _ => println!(" Should have failed\n"),
    }

    println!("Test 10: Excessive max_results (should fail)");
    match search
        .execute(json!({
            "pattern": "*.txt",
            "max_results": 2000
        }))
        .await
    {
        Ok(result) if !result.success => {
            println!(" Validation caught: {}\n", result.error.unwrap());
        }
        Err(e) => {
            println!(" Validation caught: {}\n", e);
        }
        _ => println!(" Should have failed\n"),
    }

    println!("=== All Tests Complete ===");

    Ok(())
}
