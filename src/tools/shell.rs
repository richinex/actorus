//! Shell Command Executor Tool
//!
//! Information Hiding:
//! - Command execution details (process spawning, output capture) hidden
//! - Security measures (sandboxing, timeout) hidden from caller
//! - Platform-specific implementation details abstracted

use super::{Tool, ToolMetadata, ToolParameter, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Shell command executor tool
///
/// Executes shell commands in a controlled environment with timeout protection
pub struct ShellTool {
    timeout_secs: u64,
    allowed_commands: Option<Vec<String>>,
}

impl ShellTool {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            timeout_secs,
            allowed_commands: None,
        }
    }

    pub fn with_whitelist(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = Some(commands);
        self
    }

    /// Check if command is allowed (internal implementation detail)
    fn is_command_allowed(&self, command: &str) -> bool {
        if let Some(ref allowed) = self.allowed_commands {
            // Extract the base command (first word)
            let base_cmd = command.split_whitespace().next().unwrap_or("");
            allowed.iter().any(|allowed_cmd| allowed_cmd == base_cmd)
        } else {
            true // No whitelist means all commands allowed
        }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "execute_shell".to_string(),
            description: "Execute a shell command and return its output. Use for running system commands, scripts, or CLI tools.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "The shell command to execute".to_string(),
                    required: true,
                },
            ],
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let command = args["command"].as_str().ok_or_else(|| {
            anyhow::anyhow!("'command' parameter is required and must be a string")
        })?;

        if command.is_empty() {
            return Err(anyhow::anyhow!("Command cannot be empty"));
        }

        if !self.is_command_allowed(command) {
            return Err(anyhow::anyhow!(
                "Command '{}' is not in the allowed list",
                command
            ));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let command = args["command"].as_str().unwrap();

        tracing::info!("Executing shell command: {}", command);

        // Execute with timeout protection
        let result = timeout(
            Duration::from_secs(self.timeout_secs),
            Command::new("sh").arg("-c").arg(command).output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    let combined = if stderr.is_empty() {
                        stdout.to_string()
                    } else {
                        format!("stdout:\n{}\nstderr:\n{}", stdout, stderr)
                    };
                    Ok(ToolResult::success(combined))
                } else {
                    Ok(ToolResult::failure(format!(
                        "Command failed with exit code {:?}\nstdout: {}\nstderr: {}",
                        output.status.code(),
                        stdout,
                        stderr
                    )))
                }
            }
            Ok(Err(e)) => Ok(ToolResult::failure(format!(
                "Failed to execute command: {}",
                e
            ))),
            Err(_) => Ok(ToolResult::failure(format!(
                "Command timed out after {} seconds",
                self.timeout_secs
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_shell_tool_success() {
        let tool = ShellTool::new(5);
        let args = json!({"command": "echo 'Hello, World!'"});

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_shell_tool_failure() {
        let tool = ShellTool::new(5);
        let args = json!({"command": "exit 1"});

        let result = tool.execute(args).await.unwrap();
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_shell_tool_whitelist() {
        let tool = ShellTool::new(5).with_whitelist(vec!["echo".to_string(), "ls".to_string()]);

        // Allowed command
        let args = json!({"command": "echo test"});
        let result = tool.execute(args).await;
        assert!(result.is_ok());

        // Disallowed command
        let args = json!({"command": "rm -rf /"});
        let result = tool.execute(args).await;
        assert!(result.is_err());
    }
}
