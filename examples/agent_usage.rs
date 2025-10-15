//! Example demonstrating autonomous agent capabilities
//!
//! This example shows how to use the ReAct agent to accomplish tasks
//! using available tools like shell commands, file operations, and HTTP requests.

use actorus::{agent, init};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Initialize the LLM Fusion system
    println!("Initializing LLM Fusion...");
    init().await?;
    println!("System initialized!\n");

    // Example 1: File operations
    println!("=== Example 1: Create and read a file ===");
    let result = agent::run_task(
        "Create a file called test_agent.txt with the content 'Hello from the autonomous agent!'"
    ).await?;

    if result.success {
        println!("Agent completed task successfully!");
        println!("Result: {}\n", result.result);

        println!("Agent steps:");
        for step in &result.steps {
            println!("  Iteration {}: {}", step.iteration, step.thought);
            if let Some(action) = &step.action {
                println!("    Action: {}", action);
            }
            if let Some(observation) = &step.observation {
                println!("    Observation: {}", observation);
            }
        }
    } else {
        println!("Agent failed: {:?}", result.error);
    }

    println!("\n=== Example 2: Shell command execution ===");
    let result = agent::run_task("List all .rs files in the current directory").await?;

    if result.success {
        println!("Agent result:\n{}\n", result.result);
    } else {
        println!("Agent failed: {:?}\n", result.error);
    }

    // Example 3: Multiple steps with reasoning
    println!("=== Example 3: Multi-step task with reasoning ===");
    let result = agent::run_task_with_iterations(
        "Check what day of the week it is and create a file called today.txt with that information",
        15
    ).await?;

    if result.success {
        println!("Agent completed multi-step task!");
        println!("Final result: {}", result.result);
        println!("\nThought process:");
        for step in &result.steps {
            println!("  Step {}: {}", step.iteration + 1, step.thought);
        }
    } else {
        println!("Agent failed: {:?}", result.error);
    }

    println!("\n=== Agent Demo Complete ===");

    Ok(())
}
