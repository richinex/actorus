//! Supervisor Agent Usage Example
//!
//! Demonstrates the supervisor pattern from BOOKIDEAS.md Section 12.3
//! - Multi-agent orchestration
//! - "Return ticket" pattern - agents can be invoked multiple times
//! - Complex cross-domain task decomposition

use actorus::{init, supervisor};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    println!("\n=== Supervisor Agent Example ===\n");

    init().await?;

    // Test 1: Simple multi-step task
    println!("Task 1: Count and Save");
    println!("----------------------");
    let result = supervisor::orchestrate(
        "List all Rust files in the src directory, count how many there are, \
         and write the count to a file named rust_count.txt"
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

    // Test 2: Complex coordinated task
    println!("Task 2: Research and Report");
    println!("---------------------------");
    let result = supervisor::orchestrate(
        "Create a file named status.txt, write 'System Check' to it, \
         then list all .rs files, and append the file count to status.txt"
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

    println!("=== Supervisor Agent Example Complete ===\n");

    Ok(())
}
