//! Session Usage Example
//!
//! Demonstrates how to use agent sessions for multi-turn conversations
//! with persistent context across messages.

use actorus::api::session::{self, StorageType};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    println!("=== Session Usage Example ===\n");

    // Example 1: In-memory session (ephemeral)
    println!("Example 1: In-memory session");
    println!("-----------------------------");
    {
        let mut session = session::create_session("user-123", StorageType::Memory).await?;

        // First message
        println!("User: What files are in /tmp?");
        let result = session.send_message("List files in /tmp directory").await?;
        println!("Agent: {}\n", result.result);

        // Second message - agent remembers first message context
        println!("User: How many .txt files did you find?");
        let result = session
            .send_message("From the files you just listed, count how many .txt files there are")
            .await?;
        println!("Agent: {}\n", result.result);

        println!(
            "Session has {} messages in history",
            session.message_count()
        );
    }

    println!("\n");

    // Example 2: Persistent file system session
    println!("Example 2: Persistent file system session");
    println!("------------------------------------------");
    {
        let session_path = PathBuf::from("./sessions");
        let session_id = "persistent-user";

        // First session instance
        {
            let mut session =
                session::create_session(session_id, StorageType::FileSystem(session_path.clone()))
                    .await?;

            println!("User: Remember that my favorite color is blue");
            let result = session
                .send_message("Remember: my favorite color is blue")
                .await?;
            println!("Agent: {}\n", result.result);

            println!(
                "Session persisted to disk. Message count: {}",
                session.message_count()
            );
        }

        // Second session instance - loads previous conversation
        {
            let mut session =
                session::create_session(session_id, StorageType::FileSystem(session_path.clone()))
                    .await?;

            println!("\nCreated new session instance...");
            println!("Loaded {} messages from storage", session.message_count());

            println!("\nUser: What is my favorite color?");
            let result = session
                .send_message("What is my favorite color that I told you earlier?")
                .await?;
            println!("Agent: {}\n", result.result);
        }

        // Clean up
        println!("\nCleaning up session files...");
        let mut session =
            session::create_session(session_id, StorageType::FileSystem(session_path)).await?;
        session.clear_history().await?;
        println!("Session history cleared.");
    }

    println!("\n");

    // Example 3: Multiple iterations with steps
    println!("Example 3: Complex task with steps");
    println!("-----------------------------------");
    {
        let mut session = session::create_session("demo", StorageType::Memory).await?;

        let result = session
            .send_message("Create a file called test.txt with 'Hello' in it, then read it back")
            .await?;

        println!("Result: {}\n", result.result);
        println!("Steps taken: {}", result.steps.len());
        for (i, step) in result.steps.iter().enumerate() {
            println!("  Step {}: {}", i + 1, step.thought);
            if let Some(action) = &step.action {
                println!("    Action: {}", action);
            }
        }
    }

    Ok(())
}
