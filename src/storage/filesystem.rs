//! File System Conversation Storage
//!
//! Information Hiding:
//! - File paths and JSON serialization format hidden from users
//! - Directory structure management hidden behind interface
//! - Persistence mechanism independent of storage trait users

use super::ConversationStorage;
use crate::core::llm::ChatMessage;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

/// File system storage - each session is a JSON file
/// Files are stored as {base_path}/{session_id}.json
pub struct FileSystemStorage {
    base_path: PathBuf,
}

impl FileSystemStorage {
    pub async fn new(base_path: PathBuf) -> Result<Self> {
        // Create base directory if it doesn't exist
        fs::create_dir_all(&base_path).await.context("Failed to create storage directory")?;

        Ok(Self { base_path })
    }

    fn session_path(&self, session_id: &str) -> PathBuf {
        self.base_path.join(format!("{}.json", session_id))
    }
}

#[async_trait]
impl ConversationStorage for FileSystemStorage {
    async fn save(&self, session_id: &str, history: &[ChatMessage]) -> Result<()> {
        let path = self.session_path(session_id);
        let json = serde_json::to_string_pretty(history)
            .context("Failed to serialize conversation history")?;

        fs::write(&path, json).await
            .context(format!("Failed to write session file: {:?}", path))?;

        tracing::debug!("[FileSystemStorage] Saved {} messages for session '{}' to {:?}",
                       history.len(), session_id, path);
        Ok(())
    }

    async fn load(&self, session_id: &str) -> Result<Vec<ChatMessage>> {
        let path = self.session_path(session_id);

        if !path.exists() {
            tracing::debug!("[FileSystemStorage] Session '{}' does not exist", session_id);
            return Ok(Vec::new());
        }

        let json = fs::read_to_string(&path).await
            .context(format!("Failed to read session file: {:?}", path))?;

        let history: Vec<ChatMessage> = serde_json::from_str(&json)
            .context("Failed to deserialize conversation history")?;

        tracing::debug!("[FileSystemStorage] Loaded {} messages for session '{}' from {:?}",
                       history.len(), session_id, path);
        Ok(history)
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        let path = self.session_path(session_id);

        if path.exists() {
            fs::remove_file(&path).await
                .context(format!("Failed to delete session file: {:?}", path))?;
            tracing::debug!("[FileSystemStorage] Deleted session '{}' at {:?}", session_id, path);
        } else {
            tracing::debug!("[FileSystemStorage] Session '{}' does not exist, nothing to delete", session_id);
        }

        Ok(())
    }

    async fn list_sessions(&self) -> Result<Vec<String>> {
        let mut sessions = Vec::new();
        let mut entries = fs::read_dir(&self.base_path).await
            .context("Failed to read storage directory")?;

        while let Some(entry) = entries.next_entry().await
            .context("Failed to read directory entry")? {

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(session_id) = path.file_stem().and_then(|s| s.to_str()) {
                    sessions.push(session_id.to_string());
                }
            }
        }

        tracing::debug!("[FileSystemStorage] Listed {} sessions", sessions.len());
        Ok(sessions)
    }

    async fn exists(&self, session_id: &str) -> Result<bool> {
        let path = self.session_path(session_id);
        Ok(path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path().to_path_buf()).await.unwrap();

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
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path().to_path_buf()).await.unwrap();

        let loaded = storage.load("nonexistent").await.unwrap();
        assert_eq!(loaded.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path().to_path_buf()).await.unwrap();

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
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path().to_path_buf()).await.unwrap();

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

    #[tokio::test]
    async fn test_persistence_across_instances() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Create first storage instance and save data
        {
            let storage = FileSystemStorage::new(path.clone()).await.unwrap();
            let messages = vec![ChatMessage {
                role: "user".to_string(),
                content: "Persistent message".to_string(),
            }];
            storage.save("persist-test", &messages).await.unwrap();
        }

        // Create second storage instance and load data
        {
            let storage = FileSystemStorage::new(path).await.unwrap();
            let loaded = storage.load("persist-test").await.unwrap();
            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0].content, "Persistent message");
        }
    }
}
