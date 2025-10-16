//! Advanced usage with conversations and streaming

use actorus::{chat_stream, init, Conversation};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    init().await?;

    // Multi-turn conversation
    println!("=== Conversation Example ===\n");

    let response = Conversation::new()
        .with_system("You are a helpful Rust expert")
        .user("What is ownership?")
        .send()
        .await?;

    println!("Q: What is ownership?");
    println!("A: {}\n", response);

    let response = Conversation::new()
        .with_system("You are a helpful Rust expert")
        .user("What is ownership?")
        .assistant(response)
        .user("Can you give me a simple example?")
        .send()
        .await?;

    println!("Q: Can you give me a simple example?");
    println!("A: {}\n", response);

    // Streaming example
    println!("\n=== Streaming Example ===\n");
    println!("Question: Explain async Rust in 3 sentences\n");
    println!("Answer: ");

    chat_stream("Explain async Rust in 3 sentences", |token| {
        print!("{}", token)
    })
    .await?;

    println!("\n");

    Ok(())
}
