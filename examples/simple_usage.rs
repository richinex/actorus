//! Simple usage example - just chat!

use actorus::{chat, init};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Initialize system
    init().await?;

    // Simple chat - one line!
    let response = chat("What is Rust?").await?;
    println!("Response: {}", response);

    Ok(())
}
