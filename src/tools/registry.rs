//! Tool Registry
//!
//! Information Hiding:
//! - Tool storage and lookup implementation hidden
//! - Tool lifecycle management hidden
//! - Registration and discovery mechanisms abstracted

use super::{Tool, ToolMetadata};
use std::collections::HashMap;
use std::sync::Arc;

/// Tool registry for managing available tools
///
/// Provides centralized tool management with dynamic registration
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a new tool
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.metadata().name.clone();
        tracing::info!("Registering tool: {}", name);
        self.tools.insert(name, tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get all tool metadata
    pub fn list_tools(&self) -> Vec<ToolMetadata> {
        self.tools.values().map(|tool| tool.metadata()).collect()
    }

    /// Get tool metadata as formatted string for LLM prompts
    pub fn tools_description(&self) -> String {
        let mut descriptions = Vec::new();
        for tool in self.tools.values() {
            let metadata = tool.metadata();
            let params = metadata
                .parameters
                .iter()
                .map(|p| {
                    let required = if p.required { "required" } else { "optional" };
                    format!("  - {} ({}): {} [{}]", p.name, p.param_type, p.description, required)
                })
                .collect::<Vec<_>>()
                .join("\n");

            descriptions.push(format!(
                "Tool: {}\nDescription: {}\nParameters:\n{}",
                metadata.name, metadata.description, params
            ));
        }
        descriptions.join("\n\n")
    }

    /// Create a default registry with common tools
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Register default tools
        registry.register(Arc::new(crate::tools::shell::ShellTool::new(30)));
        registry.register(Arc::new(crate::tools::filesystem::ReadFileTool::new(1024 * 1024))); // 1MB max
        registry.register(Arc::new(crate::tools::filesystem::WriteFileTool::new(1024 * 1024))); // 1MB max
        registry.register(Arc::new(crate::tools::filesystem::AppendFileTool::new(1024 * 1024))); // 1MB max
        registry.register(Arc::new(crate::tools::http::HttpTool::new(30)));

        registry
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::ShellTool;

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(ShellTool::new(10));

        registry.register(tool.clone());

        assert!(registry.has_tool("execute_shell"));
        assert!(registry.get("execute_shell").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_list_tools() {
        let registry = ToolRegistry::with_defaults();
        let tools = registry.list_tools();

        assert!(!tools.is_empty());
        assert!(registry.has_tool("execute_shell"));
        assert!(registry.has_tool("read_file"));
        assert!(registry.has_tool("write_file"));
        assert!(registry.has_tool("http_request"));
    }

    #[test]
    fn test_tools_description() {
        let registry = ToolRegistry::with_defaults();
        let description = registry.tools_description();

        assert!(description.contains("execute_shell"));
        assert!(description.contains("read_file"));
        assert!(description.contains("Description:"));
        assert!(description.contains("Parameters:"));
    }
}
