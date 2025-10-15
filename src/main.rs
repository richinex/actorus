use anyhow::Result;
use clap::Parser;
use actorus::cli::{Cli, Commands};
use actorus::{init, shutdown, utils};
use tokio::fs::File;
use tokio::io::{self, AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Initialize the system
    init().await?;

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Chat { prompt, system } => handle_chat(prompt, system).await,
        Commands::Interactive { system, memory, session_id, storage_dir } => {
            handle_interactive(system, memory, session_id, storage_dir).await
        }
        Commands::Batch { file, concurrency } => handle_batch(file, concurrency).await,
        Commands::Health { watch } => handle_health(watch).await,
    };

    // Shutdown gracefully
    shutdown().await?;

    result
}

async fn handle_chat(prompt: String, system: Option<String>) -> Result<()> {
    utils::print_info("Sending request...");

    let response = if let Some(sys) = system {
        actorus::chat_with_system(prompt, Some(sys)).await?
    } else {
        actorus::chat(prompt).await?
    };

    println!("\n{}", response);
    Ok(())
}

async fn handle_interactive(
    system: Option<String>,
    memory: bool,
    session_id: String,
    storage_dir: String,
) -> Result<()> {
    if memory {
        handle_interactive_with_memory(system, session_id, storage_dir).await
    } else {
        handle_interactive_ephemeral(system).await
    }
}

async fn handle_interactive_ephemeral(system: Option<String>) -> Result<()> {
    utils::print_header("Interactive Mode (Ephemeral)");
    utils::print_info("Type your messages (Ctrl+C to exit)");
    utils::print_info("Note: Conversation will not be saved\n");

    let mut conversation = actorus::Conversation::new();

    if let Some(sys) = system {
        conversation = conversation.with_system(sys);
    }

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);

    loop {
        utils::print_prompt("You: ");
        let mut input = String::new();
        reader.read_line(&mut input).await?;

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        conversation = conversation.user(input);

        utils::print_info("Assistant: ");
        let response = conversation.clone().send().await?;
        println!("{}\n", response);

        conversation = conversation.assistant(response);
    }
}

async fn handle_interactive_with_memory(
    system: Option<String>,
    session_id: String,
    storage_dir: String,
) -> Result<()> {
    use actorus::api::session::{self, StorageType};
    use std::path::PathBuf;

    utils::print_header("Interactive Mode (Persistent Memory)");
    utils::print_info(&format!("Session ID: {}", session_id));
    utils::print_info(&format!("Storage: {}", storage_dir));
    utils::print_info("Type your messages (Ctrl+C to exit)\n");

    // Create session with file system storage
    let mut session = session::create_session(
        session_id.clone(),
        StorageType::FileSystem(PathBuf::from(storage_dir))
    ).await?;

    // Show message count if resuming existing session
    let msg_count = session.message_count();
    if msg_count > 0 {
        utils::print_success(&format!("Resumed session with {} previous messages", msg_count));
    } else {
        utils::print_success("New session created");
    }

    // Set system prompt if provided (only for new sessions)
    if msg_count == 0 {
        if let Some(sys) = system {
            // For sessions, we add system message through the first interaction
            utils::print_info(&format!("System prompt: {}\n", sys));
            let _ = session.send_message(&format!("System context: {}", sys)).await?;
        }
    }

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);

    loop {
        utils::print_prompt("You: ");
        let mut input = String::new();
        reader.read_line(&mut input).await?;

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        // Special commands
        if input == "/clear" {
            session.clear_history().await?;
            utils::print_success("Session history cleared");
            println!();
            continue;
        }

        if input == "/count" {
            let count = session.message_count();
            utils::print_info(&format!("Messages in session: {}", count));
            println!();
            continue;
        }

        if input == "/help" {
            println!("Special commands:");
            println!("  /clear  - Clear session history");
            println!("  /count  - Show message count");
            println!("  /help   - Show this help");
            println!("  Ctrl+C  - Exit\n");
            continue;
        }

        utils::print_info("Assistant: ");
        let result = session.send_message(input).await?;
        println!("{}\n", result.result);
    }
}

async fn handle_batch(file: String, concurrency: usize) -> Result<()> {
    utils::print_info(&format!(
        "Processing prompts from {} with concurrency {}",
        file, concurrency
    ));

    let file = File::open(file).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut prompts = vec![];
    while let Some(line) = lines.next_line().await? {
        if !line.trim().is_empty() {
            prompts.push(line);
        }
    }

    let results = actorus::batch::process_prompts(prompts, concurrency).await;

    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(response) => {
                utils::print_success(&format!("\nResponse {}:", i + 1));
                println!("{}", response);
            }
            Err(e) => {
                utils::print_error(&format!("Error in prompt {}: {}", i + 1, e));
            }
        }
    }

    Ok(())
}

async fn handle_health(watch: Option<u64>) -> Result<()> {
    // Give the system a moment to start up and send initial heartbeats
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    loop {
        match actorus::get_system_state().await {
            Ok(state) => {
                println!("\n📊 System Health Status:");

                if state.active_actors.is_empty() {
                    println!(
                        "  ⚠️  No actors have sent heartbeats yet. System may be starting up..."
                    );
                } else {
                    for (actor_type, is_active) in state.active_actors.iter() {
                        let status = if *is_active {
                            "✅ Active"
                        } else {
                            "❌ Inactive"
                        };

                        let last_seen = state
                            .last_heartbeat
                            .get(actor_type)
                            .map(|instant| {
                                let elapsed = instant.elapsed();
                                if elapsed.as_secs() < 1 {
                                    format!("{}ms ago", elapsed.as_millis())
                                } else {
                                    format!("{:.1}s ago", elapsed.as_secs_f64())
                                }
                            })
                            .unwrap_or_else(|| "Never".to_string());

                        println!("  {:?}: {} (last seen: {})", actor_type, status, last_seen);
                    }
                }
                println!();
            }
            Err(e) => {
                eprintln!("❌ Failed to get system state: {}", e);
            }
        }

        // If watch mode enabled, wait and refresh
        if let Some(interval) = watch {
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            // Clear screen (works on most terminals)
            print!("\x1B[2J\x1B[1;1H");
        } else {
            break;
        }
    }

    Ok(())
}
