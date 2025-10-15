//! Filesystem Tools
//!
//! Information Hiding:
//! - File I/O implementation details hidden
//! - Path validation and security checks hidden
//! - Error handling for file operations abstracted

use super::{Tool, ToolMetadata, ToolResult};
use crate::{tool_metadata, tool_result, validate_required_string};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Read file tool
pub struct ReadFileTool {
    allowed_paths: Option<Vec<PathBuf>>,
    max_size_bytes: usize,
}

impl ReadFileTool {
    pub fn new(max_size_bytes: usize) -> Self {
        Self {
            allowed_paths: None,
            max_size_bytes,
        }
    }

    pub fn with_allowed_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.allowed_paths = Some(paths);
        self
    }

    /// Check if path is allowed (internal security check)
    fn is_path_allowed(&self, path: &Path) -> bool {
        if let Some(ref allowed) = self.allowed_paths {
            allowed.iter().any(|allowed_path| {
                path.starts_with(allowed_path) || path.canonicalize().ok()
                    .map(|p| p.starts_with(allowed_path))
                    .unwrap_or(false)
            })
        } else {
            true
        }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn metadata(&self) -> ToolMetadata {
        tool_metadata! {
            name: "read_file",
            description: "Read the contents of a file from the filesystem.",
            parameters: [
                {
                    name: "path",
                    type: "string",
                    description: "The file path to read",
                    required: true
                }
            ]
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let path_str = validate_required_string!(args, "path");

        if path_str.is_empty() {
            return Err(anyhow::anyhow!("Path cannot be empty"));
        }

        let path = Path::new(path_str);
        if !self.is_path_allowed(path) {
            return Err(anyhow::anyhow!("Access to path '{}' is not allowed", path_str));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let path_str = validate_required_string!(args, "path");
        let path = Path::new(path_str);

        tracing::info!("Reading file: {}", path_str);

        // Check file exists
        if !path.exists() {
            return Ok(ToolResult::failure(format!("File does not exist: {}", path_str)));
        }

        // Check file size
        match fs::metadata(path).await {
            Ok(metadata) => {
                let size = metadata.len() as usize;
                if size > self.max_size_bytes {
                    return Ok(ToolResult::failure(format!(
                        "File too large: {} bytes (max: {} bytes)",
                        size, self.max_size_bytes
                    )));
                }
            }
            Err(e) => return Ok(ToolResult::failure(format!("Failed to read file metadata: {}", e))),
        }

        // Read file
        match fs::read_to_string(path).await {
            Ok(contents) => tool_result!(success: contents),
            Err(e) => tool_result!(failure: format!("Failed to read file: {}", e)),
        }
    }
}

/// Write file tool
pub struct WriteFileTool {
    allowed_paths: Option<Vec<PathBuf>>,
    max_size_bytes: usize,
}

impl WriteFileTool {
    pub fn new(max_size_bytes: usize) -> Self {
        Self {
            allowed_paths: None,
            max_size_bytes,
        }
    }

    pub fn with_allowed_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.allowed_paths = Some(paths);
        self
    }

    fn is_path_allowed(&self, path: &Path) -> bool {
        if let Some(ref allowed) = self.allowed_paths {
            allowed.iter().any(|allowed_path| {
                path.starts_with(allowed_path) ||
                path.parent()
                    .and_then(|p| p.canonicalize().ok())
                    .map(|p| p.starts_with(allowed_path))
                    .unwrap_or(false)
            })
        } else {
            true
        }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn metadata(&self) -> ToolMetadata {
        tool_metadata! {
            name: "write_file",
            description: "Write content to a file on the filesystem.",
            parameters: [
                {
                    name: "path",
                    type: "string",
                    description: "The file path to write to",
                    required: true
                },
                {
                    name: "content",
                    type: "string",
                    description: "The content to write",
                    required: true
                }
            ]
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let path_str = validate_required_string!(args, "path");
        let content = validate_required_string!(args, "content");

        if path_str.is_empty() {
            return Err(anyhow::anyhow!("Path cannot be empty"));
        }

        if content.len() > self.max_size_bytes {
            return Err(anyhow::anyhow!(
                "Content too large: {} bytes (max: {} bytes)",
                content.len(),
                self.max_size_bytes
            ));
        }

        let path = Path::new(path_str);
        if !self.is_path_allowed(path) {
            return Err(anyhow::anyhow!("Access to path '{}' is not allowed", path_str));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let path_str = validate_required_string!(args, "path");
        let content = validate_required_string!(args, "content");
        let path = Path::new(path_str);

        tracing::info!("Writing to file: {}", path_str);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent).await {
                    return Ok(ToolResult::failure(format!("Failed to create directory: {}", e)));
                }
            }
        }

        // Write file
        match fs::write(path, content).await {
            Ok(_) => tool_result!(success: format!("Successfully wrote {} bytes to {}", content.len(), path_str)),
            Err(e) => tool_result!(failure: format!("Failed to write file: {}", e)),
        }
    }
}

/// Append to file tool
pub struct AppendFileTool {
    allowed_paths: Option<Vec<PathBuf>>,
    max_size_bytes: usize,
}

impl AppendFileTool {
    pub fn new(max_size_bytes: usize) -> Self {
        Self {
            allowed_paths: None,
            max_size_bytes,
        }
    }

    pub fn with_allowed_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.allowed_paths = Some(paths);
        self
    }

    fn is_path_allowed(&self, path: &Path) -> bool {
        if let Some(ref allowed) = self.allowed_paths {
            allowed.iter().any(|allowed_path| {
                path.starts_with(allowed_path) ||
                path.parent()
                    .and_then(|p| p.canonicalize().ok())
                    .map(|p| p.starts_with(allowed_path))
                    .unwrap_or(false)
            })
        } else {
            true
        }
    }
}

#[async_trait]
impl Tool for AppendFileTool {
    fn metadata(&self) -> ToolMetadata {
        tool_metadata! {
            name: "append_file",
            description: "Append content to an existing file on the filesystem. Creates the file if it doesn't exist.",
            parameters: [
                {
                    name: "path",
                    type: "string",
                    description: "The file path to append to",
                    required: true
                },
                {
                    name: "content",
                    type: "string",
                    description: "The content to append",
                    required: true
                }
            ]
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let path_str = validate_required_string!(args, "path");
        let content = validate_required_string!(args, "content");

        if path_str.is_empty() {
            return Err(anyhow::anyhow!("Path cannot be empty"));
        }

        if content.len() > self.max_size_bytes {
            return Err(anyhow::anyhow!(
                "Content too large: {} bytes (max: {} bytes)",
                content.len(),
                self.max_size_bytes
            ));
        }

        let path = Path::new(path_str);
        if !self.is_path_allowed(path) {
            return Err(anyhow::anyhow!("Access to path '{}' is not allowed", path_str));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let path_str = validate_required_string!(args, "path");
        let content = validate_required_string!(args, "content");
        let path = Path::new(path_str);

        tracing::info!("Appending to file: {}", path_str);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent).await {
                    return Ok(ToolResult::failure(format!("Failed to create directory: {}", e)));
                }
            }
        }

        // Append to file using OpenOptions
        use tokio::io::AsyncWriteExt;
        let result = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await;

        match result {
            Ok(mut file) => {
                match file.write_all(content.as_bytes()).await {
                    Ok(_) => tool_result!(success: format!(
                        "Successfully appended {} bytes to {}",
                        content.len(),
                        path_str
                    )),
                    Err(e) => tool_result!(failure: format!("Failed to write to file: {}", e)),
                }
            }
            Err(e) => tool_result!(failure: format!("Failed to open file: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_read_file_success() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").await.unwrap();

        let tool = ReadFileTool::new(1024 * 1024);
        let args = json!({"path": file_path.to_str().unwrap()});

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_file_success() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("output.txt");

        let tool = WriteFileTool::new(1024 * 1024);
        let args = json!({
            "path": file_path.to_str().unwrap(),
            "content": "Test content"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);

        let contents = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(contents, "Test content");
    }

    #[tokio::test]
    async fn test_file_size_limit() {
        let tool = ReadFileTool::new(10); // 10 bytes max

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("large.txt");
        fs::write(&file_path, "This is definitely more than 10 bytes").await.unwrap();

        let args = json!({"path": file_path.to_str().unwrap()});
        let result = tool.execute(args).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("too large"));
    }

    #[tokio::test]
    async fn test_append_file_success() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("append_test.txt");

        // First write
        let write_tool = WriteFileTool::new(1024 * 1024);
        let args = json!({
            "path": file_path.to_str().unwrap(),
            "content": "First line\n"
        });
        let result = write_tool.execute(args).await.unwrap();
        assert!(result.success);

        // Append
        let append_tool = AppendFileTool::new(1024 * 1024);
        let args = json!({
            "path": file_path.to_str().unwrap(),
            "content": "Second line\n"
        });
        let result = append_tool.execute(args).await.unwrap();
        assert!(result.success);

        // Another append
        let args = json!({
            "path": file_path.to_str().unwrap(),
            "content": "Third line\n"
        });
        let result = append_tool.execute(args).await.unwrap();
        assert!(result.success);

        // Verify contents
        let contents = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(contents, "First line\nSecond line\nThird line\n");
    }

    #[tokio::test]
    async fn test_append_creates_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("new_file.txt");

        // Append to non-existent file (should create it)
        let append_tool = AppendFileTool::new(1024 * 1024);
        let args = json!({
            "path": file_path.to_str().unwrap(),
            "content": "Created by append\n"
        });
        let result = append_tool.execute(args).await.unwrap();
        assert!(result.success);

        // Verify file was created
        assert!(file_path.exists());
        let contents = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(contents, "Created by append\n");
    }
}
