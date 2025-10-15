//! Router Agent Usage Example
//!
//! Demonstrates the router pattern from BOOKIDEAS.md Section 12.2
//! - LLM-based intent classification
//! - "One-way ticket" routing to specialized agents

use actorus::{init, router};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    println!("\n=== Router Agent Example ===\n");

    init().await?;

    // Test 1: File operations task (should route to file_ops_agent)
    println!("Task 1: Write a file");
    println!("---------------------");
    let result = router::route_task("Create a file named hello.txt with the content 'Hello from Router!'").await?;
    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);

    // Test 2: Shell command task (should route to shell_agent)
    println!("Task 2: List files");
    println!("------------------");
    let result = router::route_task("List all files in the current directory").await?;
    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);

    // Test 3: Web request task (should route to web_agent)
    println!("Task 3: Fetch web content");
    println!("--------------------------");
    let result = router::route_task("Fetch the content from https://httpbin.org/json").await?;
    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);

    // Test 4: Mixed task (should route to general_agent)
    println!("Task 4: Mixed operations");
    println!("------------------------");
    let result = router::route_task("List files and then write a summary to summary.txt").await?;
    println!("Success: {}", result.success);
    println!("Result: {}\n", result.result);

    println!("=== Router Agent Example Complete ===\n");

    Ok(())
}
