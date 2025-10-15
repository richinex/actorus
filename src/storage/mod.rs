//! Conversation Storage Abstraction
//!
//! Information Hiding:
//! - Storage backend implementation details hidden behind trait
//! - Allows swapping between memory, filesystem, SQLite, Redis without API changes
//! - Each storage implementation encapsulates its own data structures and protocols

use crate::core::llm::ChatMessage;
use anyhow::Result;
use async_trait::async_trait;

pub mod memory;
pub mod filesystem;

/// Trait defining conversation storage interface
/// Implementations can use different backends (memory, file, database, cache)
#[async_trait]
pub trait ConversationStorage: Send + Sync {
    /// Save conversation history for a session
    async fn save(&self, session_id: &str, history: &[ChatMessage]) -> Result<()>;

    /// Load conversation history for a session
    /// Returns empty vector if session doesn't exist
    async fn load(&self, session_id: &str) -> Result<Vec<ChatMessage>>;

    /// Delete conversation history for a session
    async fn delete(&self, session_id: &str) -> Result<()>;

    /// List all session IDs
    async fn list_sessions(&self) -> Result<Vec<String>>;

    /// Check if a session exists
    async fn exists(&self, session_id: &str) -> Result<bool> {
        Ok(self.load(session_id).await?.is_empty() == false)
    }
}
