//! Tool Executor with Retry Logic
//!
//! Information Hiding:
//! - Retry strategy implementation hidden
//! - Backoff algorithm hidden
//! - Error classification logic hidden

use super::{Tool, ToolConfig, ToolResult};
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Tool executor with retry and timeout support
pub struct ToolExecutor {
    config: ToolConfig,
}

impl ToolExecutor {
    pub fn new(config: ToolConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self {
            config: ToolConfig::default(),
        }
    }

    /// Execute a tool with retry logic
    pub async fn execute(&self, tool: Arc<dyn Tool>, args: Value) -> Result<ToolResult> {
        let mut last_error = None;
        let tool_name = tool.metadata().name.clone();

        for attempt in 0..self.config.max_retries {
            if attempt > 0 {
                tracing::warn!(
                    "Retrying tool '{}' (attempt {}/{})",
                    tool_name,
                    attempt + 1,
                    self.config.max_retries
                );

                // Exponential backoff
                let backoff_ms = self.calculate_backoff(attempt);
                sleep(Duration::from_millis(backoff_ms)).await;
            }

            match tool.execute(args.clone()).await {
                Ok(result) => {
                    if result.success {
                        return Ok(result);
                    } else if !self.should_retry(&result) {
                        // Don't retry on certain types of failures (e.g., validation errors)
                        return Ok(result);
                    }
                    last_error = result.error;
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                }
            }
        }

        // All retries exhausted
        Ok(ToolResult::failure(format!(
            "Tool '{}' failed after {} attempts. Last error: {}",
            tool_name,
            self.config.max_retries,
            last_error.unwrap_or_else(|| "Unknown error".to_string())
        )))
    }

    /// Calculate exponential backoff delay (internal implementation)
    fn calculate_backoff(&self, attempt: u32) -> u64 {
        let base_delay = 100; // 100ms base
        let max_delay = 5000; // 5s max

        let delay = base_delay * 2_u64.pow(attempt);
        delay.min(max_delay)
    }

    /// Determine if error is retryable (internal logic)
    fn should_retry(&self, result: &ToolResult) -> bool {
        if let Some(ref error) = result.error {
            let error_lower = error.to_lowercase();

            // Don't retry validation errors or permission issues
            if error_lower.contains("validation")
                || error_lower.contains("not allowed")
                || error_lower.contains("permission")
                || error_lower.contains("empty")
            {
                return false;
            }

            // Retry timeouts and network errors
            if error_lower.contains("timeout")
                || error_lower.contains("connection")
                || error_lower.contains("network")
            {
                return true;
            }
        }

        // Default: retry
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{Tool, ToolMetadata, ToolResult};
    use async_trait::async_trait;

    struct MockTool {
        fail_count: std::sync::Mutex<u32>,
        max_fails: u32,
    }

    impl MockTool {
        fn new(max_fails: u32) -> Self {
            Self {
                fail_count: std::sync::Mutex::new(0),
                max_fails,
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn metadata(&self) -> ToolMetadata {
            ToolMetadata {
                name: "mock_tool".to_string(),
                description: "Mock tool for testing".to_string(),
                parameters: vec![],
            }
        }

        async fn execute(&self, _args: Value) -> Result<ToolResult> {
            let mut count = self.fail_count.lock().unwrap();
            *count += 1;

            if *count <= self.max_fails {
                Ok(ToolResult::failure("Temporary failure"))
            } else {
                Ok(ToolResult::success("Success after retries"))
            }
        }
    }

    #[tokio::test]
    async fn test_executor_retry_success() {
        let executor = ToolExecutor::new(ToolConfig {
            timeout_secs: 30,
            max_retries: 3,
            sandbox: false,
        });

        let tool = Arc::new(MockTool::new(2)); // Fail twice, then succeed
        let result = executor.execute(tool, serde_json::json!({})).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("Success after retries"));
    }

    #[tokio::test]
    async fn test_executor_retry_exhausted() {
        let executor = ToolExecutor::new(ToolConfig {
            timeout_secs: 30,
            max_retries: 2,
            sandbox: false,
        });

        let tool = Arc::new(MockTool::new(5)); // Will keep failing
        let result = executor.execute(tool, serde_json::json!({})).await.unwrap();

        assert!(!result.success);
        assert!(result.error.unwrap().contains("failed after"));
    }
}
