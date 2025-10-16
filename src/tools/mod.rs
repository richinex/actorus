//! Tool System - Provides extensible tool execution for agents
//!
//! Information Hiding:
//! - Tool execution details hidden behind trait
//! - Tool parameters and schemas hidden in implementations
//! - Registry implementation details hidden from consumers
//! - Error handling internalized per tool

pub mod executor;
pub mod filesystem;
pub mod http;
pub mod macros;
pub mod registry;
pub mod shell;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Tool parameter schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// Tool metadata - describes what the tool does and how to use it
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl fmt::Display for ToolMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.description)
    }
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
        }
    }
}

/// Tool trait - All tools must implement this
///
/// Information Hiding: Tool implementations hide their internal execution logic,
/// data structures, and error handling strategies behind this interface.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get tool metadata (name, description, parameters)
    fn metadata(&self) -> ToolMetadata;

    /// Execute the tool with given arguments
    ///
    /// # Arguments
    /// * `args` - JSON value containing tool arguments
    ///
    /// # Returns
    /// * `ToolResult` - Success or failure with output/error
    async fn execute(&self, args: Value) -> Result<ToolResult>;

    /// Validate arguments before execution (optional)
    fn validate(&self, _args: &Value) -> Result<()> {
        Ok(())
    }
}

/// Tool execution configuration
#[derive(Debug, Clone)]
pub struct ToolConfig {
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub sandbox: bool,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_retries: 3,
            sandbox: true,
        }
    }
}
