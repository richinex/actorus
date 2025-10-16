//! In-Memory Conversation Storage
//!
//! Information Hiding:
//! - HashMap storage structure hidden from users
//! - Thread-safe access via RwLock hidden behind async interface
//! - Suitable for testing and ephemeral sessions

use super::ConversationStorage;
use crate::core::llm::ChatMessage;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory storage using HashMap
/// Data is lost when process terminates
pub struct InMemoryStorage {
    sessions: Arc<RwLock<HashMap<String, Vec<ChatMessage>>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConversationStorage for InMemoryStorage {
    async fn save(&self, session_id: &str, history: &[ChatMessage]) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), history.to_vec());
        tracing::debug!(
            "[InMemoryStorage] Saved {} messages for session '{}'",
            history.len(),
            session_id
        );
        Ok(())
    }

    async fn load(&self, session_id: &str) -> Result<Vec<ChatMessage>> {
        let sessions = self.sessions.read().await;
        let history = sessions.get(session_id).cloned().unwrap_or_default();
        tracing::debug!(
            "[InMemoryStorage] Loaded {} messages for session '{}'",
            history.len(),
            session_id
        );
        Ok(history)
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        tracing::debug!("[InMemoryStorage] Deleted session '{}'", session_id);
        Ok(())
    }

    async fn list_sessions(&self) -> Result<Vec<String>> {
        let sessions = self.sessions.read().await;
        let session_ids: Vec<String> = sessions.keys().cloned().collect();
        tracing::debug!("[InMemoryStorage] Listed {} sessions", session_ids.len());
        Ok(session_ids)
    }

    async fn exists(&self, session_id: &str) -> Result<bool> {
        let sessions = self.sessions.read().await;
        Ok(sessions.contains_key(session_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load() {
        let storage = InMemoryStorage::new();
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: "Hi there".to_string(),
            },
        ];

        storage.save("test-session", &messages).await.unwrap();
        let loaded = storage.load("test-session").await.unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].content, "Hello");
        assert_eq!(loaded[1].content, "Hi there");
    }

    #[tokio::test]
    async fn test_load_nonexistent_session() {
        let storage = InMemoryStorage::new();
        let loaded = storage.load("nonexistent").await.unwrap();
        assert_eq!(loaded.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let storage = InMemoryStorage::new();
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "Test".to_string(),
        }];

        storage.save("test-session", &messages).await.unwrap();
        assert!(storage.exists("test-session").await.unwrap());

        storage.delete("test-session").await.unwrap();
        assert!(!storage.exists("test-session").await.unwrap());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let storage = InMemoryStorage::new();
        let msg = vec![ChatMessage {
            role: "user".to_string(),
            content: "Test".to_string(),
        }];

        storage.save("session-1", &msg).await.unwrap();
        storage.save("session-2", &msg).await.unwrap();

        let sessions = storage.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&"session-1".to_string()));
        assert!(sessions.contains(&"session-2".to_string()));
    }
}
