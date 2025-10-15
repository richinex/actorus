//! Simple test to see autonomous agent in action

use actorus::{agent, init};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    println!("\n Starting Autonomous Agent Test\n");

    init().await?;

    // Simple task that will definitely work
    println!("📋 Task: List files in current directory\n");

    let result =
        agent::run_task("Use shell command to list all files in the current directory").await?;

    println!("\n✨ RESULT:");
    println!("Success: {}", result.success);
    println!("\n📄 Output:\n{}\n", result.result);

    println!("🧠 Agent's Reasoning ({} steps):", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        println!("\nStep {}:", i + 1);
        println!("  💭 Thought: {}", step.thought);
        if let Some(tool) = &step.action {
            println!("  🔧 Tool Used: {}", tool);
        }
        if let Some(obs) = &step.observation {
            println!(
                "  👁️  Observation: {}",
                obs.chars().take(100).collect::<String>()
            );
        }
    }

    Ok(())
}
