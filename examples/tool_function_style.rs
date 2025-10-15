//! Example: Function-style tool definition (MCP/Python familiar)
//!
//! Demonstrates the #[tool_fn] macro which provides a Python/MCP-like
//! function-based API while generating the Rust struct internally.
//!
//! This is the MOST FAMILIAR style for developers coming from Python!

// Note: The #[tool_fn] macro generates code that calls these functions from the
// Tool implementation. Rustc shows "unused variable" warnings at the source location
// before macro expansion, but the variables ARE used. This suppresses those warnings.
#![allow(unused_variables)]

use anyhow::Result;
use actorus::tool_fn;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A simple greeting tool - just define a function!
#[tool_fn(name = "greet", description = "Greet a person with a custom message")]
async fn greet(name: String, greeting: Option<String>) -> Result<String> {
    let greeting = greeting.unwrap_or_else(|| "Hello".to_string());
    Ok(format!("{}, {}!", greeting, name))
}

/// Calculate arithmetic operations
#[tool_fn(
    name = "calculate",
    description = "Perform basic arithmetic operations"
)]
async fn calculate(operation: String, a: i64, b: i64) -> Result<String> {
    let result = match operation.as_str() {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        "divide" => {
            if b == 0 {
                return Err(anyhow::anyhow!("Cannot divide by zero"));
            }
            a / b
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid operation. Use: add, subtract, multiply, divide"
            ))
        }
    };

    Ok(format!("{} {} {} = {}", a, operation, b, result))
}

/// Format text with various transformations
#[tool_fn(
    name = "format_text",
    description = "Format text with optional transformations"
)]
async fn format_text(
    text: String,
    uppercase: Option<bool>,
    prefix: Option<String>,
    suffix: Option<String>,
) -> Result<String> {
    let mut result = text;

    if uppercase.unwrap_or(false) {
        result = result.to_uppercase();
    }

    if let Some(p) = prefix {
        result = format!("{}{}", p, result);
    }

    if let Some(s) = suffix {
        result = format!("{}{}", result, s);
    }

    Ok(result)
}

/// User information struct
#[derive(Debug, Serialize, Deserialize)]
struct UserInfo {
    name: String,
    age: i32,
    email: String,
}

/// Process user information - demonstrates struct parameters
#[tool_fn(
    name = "process_user",
    description = "Process user information from a struct"
)]
async fn process_user(user: UserInfo) -> Result<String> {
    Ok(format!(
        "User: {} (age: {}, email: {})",
        user.name, user.age, user.email
    ))
}

/// Operation enum for demonstration
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Calculate with enum - demonstrates enum parameters
#[tool_fn(
    name = "calculate_enum",
    description = "Calculate with operation enum. Valid ops: add, subtract, multiply, divide (lowercase)"
)]
async fn calculate_enum(op: Operation, a: i64, b: i64) -> Result<String> {
    let result = match op {
        Operation::Add => a + b,
        Operation::Subtract => a - b,
        Operation::Multiply => a * b,
        Operation::Divide => {
            if b == 0 {
                return Err(anyhow::anyhow!("Cannot divide by zero"));
            }
            a / b
        }
    };
    Ok(format!("{:?}: {} and {} = {}", op, a, b, result))
}

#[tokio::main]
async fn main() -> Result<()> {
    use actorus::tools::Tool;

    println!("=== Function-Style Tool Definition (MCP/Python Familiar) ===\n");
    println!("This style is familiar to Python developers and MCP users!");
    println!("Just write a function with #[tool_fn] and you're done.\n");

    // The macro auto-generates a GreetTool struct
    println!("--- GreetTool (from greet function) ---");
    let greet_tool = GreetTool::new();

    let metadata = greet_tool.metadata();
    println!("Tool: {}", metadata.name);
    println!("Description: {}", metadata.description);
    println!("Parameters:");
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

    println!("\n--- Test 1: Default greeting ---");
    let result = greet_tool.execute(json!({"name": "Alice"})).await?;
    println!("Result: {}\n", result.output);

    println!("--- Test 2: Custom greeting ---");
    let result = greet_tool
        .execute(json!({
            "name": "Bob",
            "greeting": "Hi there"
        }))
        .await?;
    println!("Result: {}\n", result.output);

    // The macro auto-generates a CalculateTool struct
    println!("\n--- CalculateTool (from calculate function) ---");
    let calc_tool = CalculateTool::new();

    let metadata = calc_tool.metadata();
    println!("Tool: {}", metadata.name);
    println!("Description: {}", metadata.description);

    println!("\n--- Test 1: Addition ---");
    let result = calc_tool
        .execute(json!({
            "operation": "add",
            "a": 10,
            "b": 5
        }))
        .await?;
    println!("Result: {}\n", result.output);

    println!("--- Test 2: Division ---");
    let result = calc_tool
        .execute(json!({
            "operation": "divide",
            "a": 20,
            "b": 4
        }))
        .await?;
    println!("Result: {}\n", result.output);

    println!("--- Test 3: Division by zero (error handling) ---");
    match calc_tool
        .execute(json!({
            "operation": "divide",
            "a": 10,
            "b": 0
        }))
        .await
    {
        Ok(result) if !result.success => {
            println!(" Error caught: {}\n", result.error.unwrap());
        }
        Err(e) => {
            println!(" Error caught: {}\n", e);
        }
        _ => println!(" Should have failed\n"),
    }

    // The macro auto-generates a FormatTextTool struct
    println!("\n--- FormatTextTool (from format_text function) ---");
    let format_tool = FormatTextTool::new();

    println!("--- Test 1: Basic text ---");
    let result = format_tool
        .execute(json!({
            "text": "hello world"
        }))
        .await?;
    println!("Result: {}\n", result.output);

    println!("--- Test 2: Uppercase with prefix ---");
    let result = format_tool
        .execute(json!({
            "text": "hello",
            "uppercase": true,
            "prefix": ">>> "
        }))
        .await?;
    println!("Result: {}\n", result.output);

    println!("--- Test 3: All options ---");
    let result = format_tool
        .execute(json!({
            "text": "content",
            "uppercase": true,
            "prefix": "[",
            "suffix": "]"
        }))
        .await?;
    println!("Result: {}\n", result.output);

    // Test struct parameters
    println!("\n--- ProcessUserTool (struct parameter) ---");
    let user_tool = ProcessUserTool::new();

    println!("--- Test: Struct parameter ---");
    let result = user_tool
        .execute(json!({
            "user": {
                "name": "Charlie",
                "age": 25,
                "email": "charlie@example.com"
            }
        }))
        .await?;
    println!("Result: {}\n", result.output);

    // Test enum parameters
    println!("\n--- CalculateEnumTool (enum parameter) ---");
    let enum_tool = CalculateEnumTool::new();

    println!("--- Test: Enum parameter (lowercase string) ---");
    let result = enum_tool
        .execute(json!({
            "op": "multiply",
            "a": 7,
            "b": 6
        }))
        .await?;
    println!("Result: {}\n", result.output);

    println!("\n=== Key Advantages ===");
    println!("1.  Familiar to Python/MCP developers");
    println!("2.  Just write a function - macro does the rest");
    println!("3.  Type inference from function signature");
    println!("4.  Required vs optional from Option<T>");
    println!("5.  Supports structs, enums (just need Serialize/Deserialize)");
    println!("6.  Auto-generates struct, metadata, validation");
    println!("7.  Clean, minimal boilerplate - ONE macro does it all");
    println!("\n=== When to Use ===");
    println!("- Simple, stateless tools");
    println!("- Familiar API for Python developers");
    println!("- Quick tool prototyping");
    println!("\n=== When to Use Struct Style Instead ===");
    println!("- Tools need configuration/state");
    println!("- Complex custom validation");
    println!("- Same tool, different configs");

    Ok(())
}
