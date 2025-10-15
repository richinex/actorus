//! Batch processing example

use actorus::{init, batch};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    init().await?;

    let prompts = vec![
        "What is Rust?".to_string(),
        "What is async?".to_string(),
        "What are actors?".to_string(),
        "What is tokio?".to_string(),
        "What is MCP?".to_string(),
    ];

    println!("Processing {} prompts with concurrency 3...\n", prompts.len());

    let start = std::time::Instant::now();
    let results = batch::process_prompts(prompts.clone(), 3).await;
    let elapsed = start.elapsed();

    for (i, result) in results.iter().enumerate() {
        println!("Prompt {}: {}", i + 1, prompts[i]);
        match result {
            Ok(response) => {
                println!("Response: {}\n", response.chars().take(100).collect::<String>());
            }
            Err(e) => {
                println!("Error: {}\n", e);
            }
        }
    }

    println!("Total time: {:?}", elapsed);
    println!("Average per request: {:?}", elapsed / prompts.len() as u32);

    Ok(())
}
