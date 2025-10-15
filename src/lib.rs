//! Actorus - Actor-based multi-agent system for LLM orchestration
//!
//! This library provides an actor-based architecture for building
//! fault-tolerant multi-agent LLM systems with MCP integration.

// Re-export procedural macros
pub use actorus_macros::{tool, tool_fn};

pub mod actors;
mod config;
pub mod core; // Make core public for MCP access
pub mod storage;
pub mod tools;
pub mod utils;

pub mod api;
pub mod cli;

pub use api::*;
pub use config::Settings;

// ✅ Re-export StateSnapshot for public use
pub use actors::messages::StateSnapshot;

// ✅ Re-export AgentBuilder for easy agent creation
pub use actors::{AgentBuilder, AgentCollection};

// ✅ Re-export ResponseFormat for structured outputs
pub use core::llm::{JsonSchemaFormat, ResponseFormat};

use actors::MessageRouterHandle;
use once_cell::sync::OnceCell;
use tokio::sync::oneshot;

static SYSTEM: OnceCell<System> = OnceCell::new();

pub struct System {
    router: MessageRouterHandle,
}

impl System {
    fn new(settings: Settings, api_key: String) -> Self {
        Self {
            router: MessageRouterHandle::new(settings, api_key),
        }
    }

    fn global() -> &'static System {
        SYSTEM
            .get()
            .expect("System not initialized. Call init() first")
    }
}

/// Initialize the system
/// Must be called before using any API functions
pub async fn init() -> anyhow::Result<()> {
    let settings = Settings::new()?;
    let api_key = Settings::api_key()?;

    let system = System::new(settings, api_key);
    SYSTEM
        .set(system)
        .map_err(|_| anyhow::anyhow!("System already initialized"))?;

    tracing::info!("Actorus system initialized");
    Ok(())
}

/// Shutdown the system gracefully
pub async fn shutdown() -> anyhow::Result<()> {
    if let Some(system) = SYSTEM.get() {
        system.router.shutdown().await?;
    }
    tracing::info!("Actorus system shutdown complete");
    Ok(())
}

/// Get the current state of the actor system
/// Returns a snapshot showing which actors are active and their last heartbeat times
pub async fn get_system_state() -> anyhow::Result<StateSnapshot> {
    let system = System::global();

    let (response_tx, response_rx) = oneshot::channel();

    system
        .router
        .send_message(actors::messages::RoutingMessage::GetState(response_tx))
        .await?;

    response_rx
        .await
        .map_err(|e| anyhow::anyhow!("Failed to receive system state: {}", e))
}
