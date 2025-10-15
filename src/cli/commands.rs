use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "llm-fusion")]
#[command(author, version, about = "Simple async API for LLM interactions", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Send a single chat message
    Chat {
        prompt: String,

        #[arg(short = 's', long)]
        system: Option<String>,
    },

    /// Start an interactive chat session
    Interactive {
        #[arg(short = 's', long)]
        system: Option<String>,

        /// Enable persistent memory (saves conversation to disk)
        #[arg(short = 'm', long)]
        memory: bool,

        /// Session ID for persistent memory (default: "default")
        #[arg(long, default_value = "default")]
        session_id: String,

        /// Storage directory for persistent memory (default: "./sessions")
        #[arg(long, default_value = "./sessions")]
        storage_dir: String,
    },

    /// Process prompts from a file in batch
    Batch {
        file: String,

        #[arg(short, long, default_value = "5")]
        concurrency: usize,
    },

    /// Check the health status of all actors in the system
    Health {
        /// Enable continuous monitoring (refresh every N seconds)
        #[arg(short, long)]
        watch: Option<u64>,
    },
}
