//! Interactive Session Example
//!
//! Demonstrates persistent multi-turn conversations using agent sessions.
//! The agent maintains context across messages, allowing for natural follow-up questions.

use actorus::api::session::{self, StorageType};
use std::io::{self, Write};
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

    println!("=== LLM Fusion Interactive Session ===\n");
    println!("This example demonstrates persistent multi-turn conversations.");
    println!("The agent will remember context from previous messages.\n");

    // Choose storage type
    println!("Choose storage type:");
    println!("1. In-memory (lost on exit)");
    println!("2. File system (persists to disk)");
    print!("Enter choice [1-2]: ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;

    let storage_type = match choice.trim() {
        "2" => StorageType::FileSystem(PathBuf::from("./sessions")),
        _ => StorageType::Memory,
    };

    // Create session
    print!("\nEnter session ID (e.g., 'user-123'): ");
    io::stdout().flush()?;

    let mut session_id = String::new();
    io::stdin().read_line(&mut session_id)?;
    let session_id = session_id.trim();

    println!("\nCreating session '{}'...", session_id);
    let mut session = session::create_session(session_id, storage_type).await?;

    println!("Session created!");
    if session.message_count() > 0 {
        println!(
            "Loaded existing conversation with {} messages",
            session.message_count()
        );
    }

    println!("\nCommands:");
    println!("  /clear    - Clear conversation history");
    println!("  /history  - Show message count");
    println!("  /help     - Show this help");
    println!("  /exit     - Exit the session");
    println!("\nType your message and press Enter:\n");

    // Interactive loop
    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match input {
            "/exit" => {
                println!("Exiting session. Goodbye!");
                break;
            }
            "/clear" => {
                session.clear_history().await?;
                println!("Conversation history cleared.");
                continue;
            }
            "/history" => {
                println!("Messages in conversation: {}", session.message_count());
                continue;
            }
            "/help" => {
                println!("\nCommands:");
                println!("  /clear    - Clear conversation history");
                println!("  /history  - Show message count");
                println!("  /help     - Show this help");
                println!("  /exit     - Exit the session");
                println!();
                continue;
            }
            _ => {}
        }

        // Send message to agent
        println!("\nProcessing...");
        match session.send_message(input).await {
            Ok(result) => {
                if result.success {
                    println!("\nAgent: {}\n", result.result);

                    // Show steps if requested
                    if input.contains("show steps") && !result.steps.is_empty() {
                        println!("Steps taken:");
                        for (i, step) in result.steps.iter().enumerate() {
                            println!("  Step {}: {}", i + 1, step.thought);
                            if let Some(action) = &step.action {
                                println!("    Action: {}", action);
                            }
                            if let Some(obs) = &step.observation {
                                println!("    Result: {}", obs);
                            }
                        }
                        println!();
                    }
                } else {
                    println!("\nAgent failed: {:?}\n", result.error);
                }
            }
            Err(e) => {
                println!("\nError: {}\n", e);
            }
        }
    }

    Ok(())
}
