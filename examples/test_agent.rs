//! Comprehensive test of the autonomous agent system
//!
//! This example tests all major features to verify the system is working

use actorus::{agent, init, shutdown};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    println!("");
    println!("   LLM Fusion Agent System - Comprehensive Test          ");
    println!("\n");

    // Test 1: System Initialization
    println!("🔧 Test 1: System Initialization");
    println!("   Initializing LLM Fusion system...");
    init().await?;
    println!("   ✅ System initialized successfully\n");

    // Test 2: Simple file write task
    println!("📝 Test 2: File Write Task");
    println!("   Task: Create a file called agent_test.txt with 'Hello from Agent!'");

    match agent::run_task("Create a file called agent_test.txt with the content 'Hello from Agent!'").await {
        Ok(result) => {
            if result.success {
                println!("   ✅ Task completed successfully!");
                println!("   Result: {}", result.result);
                println!("   Steps taken: {}", result.steps.len());

                // Verify the file exists
                if std::path::Path::new("agent_test.txt").exists() {
                    println!("   ✅ File verified on disk");
                } else {
                    println!("     File not found on disk");
                }
            } else {
                println!("   ❌ Task failed: {:?}", result.error);
            }
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }
    println!();

    // Test 3: File read task
    println!("📖 Test 3: File Read Task");
    println!("   Task: Read the contents of agent_test.txt");

    match agent::run_task("Read the contents of the file agent_test.txt").await {
        Ok(result) => {
            if result.success {
                println!("   ✅ Read successful!");
                println!("   Content preview: {}",
                    result.result.chars().take(100).collect::<String>());
                println!("   Steps taken: {}", result.steps.len());
            } else {
                println!("   ❌ Read failed: {:?}", result.error);
            }
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }
    println!();

    // Test 4: Shell command execution
    println!("💻 Test 4: Shell Command Execution");
    println!("   Task: List Rust files in current directory");

    match agent::run_task("List all .rs files in the current directory").await {
        Ok(result) => {
            if result.success {
                println!("   ✅ Command executed!");
                println!("   Files found: {}",
                    result.result.lines().take(5).collect::<Vec<_>>().join("\n   "));
                println!("   Steps taken: {}", result.steps.len());
            } else {
                println!("   ❌ Command failed: {:?}", result.error);
            }
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }
    println!();

    // Test 5: Multi-step reasoning
    println!("🧠 Test 5: Multi-Step Reasoning");
    println!("   Task: Count words in agent_test.txt and create a report");

    match agent::run_task_with_iterations(
        "Read agent_test.txt, count the words, and create a new file called word_count.txt with the count",
        15
    ).await {
        Ok(result) => {
            if result.success {
                println!("   ✅ Multi-step task completed!");
                println!("   Result: {}", result.result);
                println!("   Total steps: {}", result.steps.len());

                // Show the reasoning process
                println!("\n   Agent's thought process:");
                for (i, step) in result.steps.iter().enumerate() {
                    println!("   Step {}: {}", i + 1, step.thought.chars().take(60).collect::<String>());
                    if let Some(action) = &step.action {
                        println!("      → Tool: {}", action);
                    }
                }
            } else {
                println!("   ❌ Task failed: {:?}", result.error);
                println!("   Partial steps: {}", result.steps.len());
            }
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }
    println!();

    // Test 6: Agent iteration control
    println!("⏱️  Test 6: Iteration Control");
    println!("   Task: Simple task with only 3 iterations allowed");

    match agent::run_task_with_iterations(
        "Create a file called iteration_test.txt with 'Test'",
        3
    ).await {
        Ok(result) => {
            println!("   Result: {} (used {} iterations)",
                if result.success { "✅ Success" } else { "❌ Failed" },
                result.steps.len());
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }
    println!();

    // Test 7: Error handling
    println!("  Test 7: Error Handling");
    println!("   Task: Try to read a non-existent file");

    match agent::run_task("Read the contents of /nonexistent/file.txt").await {
        Ok(result) => {
            if !result.success {
                println!("   ✅ Error handled gracefully!");
                println!("   Error message: {:?}", result.error);
            } else {
                println!("     Unexpectedly succeeded?");
            }
        }
        Err(e) => {
            println!("   ✅ Error caught at API level: {}", e);
        }
    }
    println!();

    // Test 8: Agent stop
    println!("🛑 Test 8: Agent Lifecycle - Stop");
    println!("   Stopping agent actor...");

    match agent::stop().await {
        Ok(_) => println!("   ✅ Agent stopped successfully"),
        Err(e) => println!("   ❌ Stop failed: {}", e),
    }
    println!();

    // Test 9: System shutdown
    println!("🔌 Test 9: System Shutdown");
    println!("   Shutting down LLM Fusion system...");

    match shutdown().await {
        Ok(_) => println!("   ✅ System shutdown complete"),
        Err(e) => println!("   ❌ Shutdown failed: {}", e),
    }
    println!();

    // Cleanup
    println!("🧹 Cleanup");
    let _ = std::fs::remove_file("agent_test.txt");
    let _ = std::fs::remove_file("word_count.txt");
    let _ = std::fs::remove_file("iteration_test.txt");
    println!("   ✅ Test files cleaned up\n");

    println!("");
    println!("   Test Suite Complete!                                   ");
    println!("");

    Ok(())
}
