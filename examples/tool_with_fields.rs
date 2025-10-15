//! Example: Tool with struct fields using proc macro
//!
//! Demonstrates how the macro handles structs with both:
//! - Config fields (not exposed to LLM)
//! - Parameter fields (exposed to LLM with #[param])

use actorus::tool;
use actorus::tools::{Tool, ToolMetadata, ToolResult};
use actorus::{
    tool_result, validate_optional_string, validate_required_number, validate_required_string,
};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// A calculator tool with configuration
pub struct CalculatorTool {
    // Config field - NOT exposed to LLM
    max_precision: usize,
    // These would be parameter fields if we add #[param] attributes
    // For now, parameters come from args in execute()
}

// Apply macro to generate metadata helper
#[tool(
    name = "calculate",
    description = "Perform arithmetic operations with precision control"
)]
impl CalculatorTool {}

impl CalculatorTool {
    pub fn new(max_precision: usize) -> Self {
        Self { max_precision }
    }
}

#[async_trait]
impl Tool for CalculatorTool {
    fn metadata(&self) -> ToolMetadata {
        Self::tool_metadata() // Generated helper
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let operation = validate_required_string!(args, "operation");
        let _a = validate_required_number!(args, "a");
        let b = validate_required_number!(args, "b");

        // Custom validation using struct field
        if !["add", "subtract", "multiply", "divide"].contains(&operation) {
            return Err(anyhow::anyhow!("Invalid operation: {}", operation));
        }

        if operation == "divide" && b == 0 {
            return Err(anyhow::anyhow!("Cannot divide by zero"));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let operation = validate_required_string!(args, "operation");
        let a = validate_required_number!(args, "a") as f64;
        let b = validate_required_number!(args, "b") as f64;

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => a / b,
            _ => unreachable!(),
        };

        // Use the config field to format with precision
        let formatted = format!("{:.precision$}", result, precision = self.max_precision);

        tool_result!(success: format!("{} {} {} = {}", a, operation, b, formatted))
    }
}

/// A file processor with size limits
pub struct FileProcessorTool {
    // Config fields - internal configuration
    max_size_mb: usize,
    allowed_extensions: Vec<String>,
}

// Generate metadata helper
#[tool(name = "process_file", description = "Process a file with size limits")]
impl FileProcessorTool {}

impl FileProcessorTool {
    pub fn new(max_size_mb: usize, allowed_extensions: Vec<String>) -> Self {
        Self {
            max_size_mb,
            allowed_extensions,
        }
    }
}

#[async_trait]
impl Tool for FileProcessorTool {
    fn metadata(&self) -> ToolMetadata {
        Self::tool_metadata()
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let path = validate_required_string!(args, "path");

        // Custom validation using struct fields
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        if !self.allowed_extensions.contains(&extension.to_string()) {
            return Err(anyhow::anyhow!(
                "File type '{}' not allowed. Allowed: {:?}",
                extension,
                self.allowed_extensions
            ));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let path = validate_required_string!(args, "path");
        let action = validate_optional_string!(args, "action", "analyze");

        // Use struct fields in logic
        let message = format!(
            "Processing '{}' with action '{}' (max size: {}MB, allowed types: {:?})",
            path, action, self.max_size_mb, self.allowed_extensions
        );

        tool_result!(success: message)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    use serde_json::json;

    println!("=== Tools with Struct Fields Example ===\n");

    // Example 1: CalculatorTool with precision config
    println!("--- Calculator Tool (max_precision: 2) ---");
    let calc = CalculatorTool::new(2);

    let metadata = calc.metadata();
    println!("Tool: {}", metadata.name);
    println!("Description: {}", metadata.description);

    let result = calc
        .execute(json!({
            "operation": "divide",
            "a": 10,
            "b": 3
        }))
        .await?;
    println!("Result: {}\n", result.output);

    // Same tool with different precision
    println!("--- Calculator Tool (max_precision: 5) ---");
    let calc_precise = CalculatorTool::new(5);
    let result = calc_precise
        .execute(json!({
            "operation": "divide",
            "a": 10,
            "b": 3
        }))
        .await?;
    println!("Result: {}\n", result.output);

    // Example 2: FileProcessorTool with extension restrictions
    println!("--- File Processor Tool ---");
    let processor = FileProcessorTool::new(
        10,
        vec!["txt".to_string(), "json".to_string(), "md".to_string()],
    );

    let metadata = processor.metadata();
    println!("Tool: {}", metadata.name);
    println!("Description: {}", metadata.description);

    println!("\nTest 1: Valid extension");
    let result = processor
        .execute(json!({
            "path": "document.txt",
            "action": "analyze"
        }))
        .await?;
    println!(" {}\n", result.output);

    println!("Test 2: Invalid extension");
    match processor
        .execute(json!({
            "path": "image.png"
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
    Ok(())
}
