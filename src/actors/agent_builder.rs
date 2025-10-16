//! Agent Builder - Simplified API for creating specialized agents
//!
//! Information Hiding:
//! - Hides tool registration complexity
//! - Encapsulates Arc wrapping details
//! - Internal agent configuration management
//! - Exposes fluent builder interface

use crate::tools::Tool;
use std::sync::Arc;

/// Type alias for agent configuration tuple
/// Format: (name, description, system_prompt, tools, response_schema)
pub type AgentConfig = (
    String,
    String,
    String,
    Vec<Arc<dyn Tool>>,
    Option<serde_json::Value>,
);

/// Builder for creating specialized agent configurations
///
/// Provides a fluent API for constructing agents with custom tools
/// and configuration. Hides the complexity of Arc wrapping and
/// tuple-based configuration.
///
/// # Example
/// ```no_run
/// use actorus::AgentBuilder;
///
/// let agent = AgentBuilder::new("data_agent")
///     .description("Manages inventory data")
///     .system_prompt("You are a data management specialist")
///     .tool(AddItemTool::new())
///     .tool(SearchItemsTool::new())
///     .build();
/// ```
pub struct AgentBuilder {
    name: String,
    description: Option<String>,
    system_prompt: Option<String>,
    tools: Vec<Arc<dyn Tool>>,
    response_schema: Option<serde_json::Value>,
    return_tool_output: bool,
}

impl AgentBuilder {
    /// Create a new agent builder with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            system_prompt: None,
            tools: Vec::new(),
            response_schema: None,
            return_tool_output: false,
        }
    }

    /// Set the agent's description
    ///
    /// This is used by routers and supervisors to understand what the agent can do.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the agent's system prompt
    ///
    /// This guides the agent's behavior and decision-making.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add a tool to the agent
    ///
    /// Tools are automatically Arc-wrapped for shared ownership.
    pub fn tool<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.push(Arc::new(tool));
        self
    }

    /// Add multiple tools at once
    pub fn tools<T: Tool + 'static>(mut self, tools: Vec<T>) -> Self {
        for tool in tools {
            self.tools.push(Arc::new(tool));
        }
        self
    }

    /// Add a pre-wrapped Arc<dyn Tool>
    ///
    /// Useful when you already have Arc-wrapped tools or dynamic tools.
    pub fn tool_arc(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Set the response schema for structured outputs
    ///
    /// When set, the agent will use OpenAI's Structured Outputs feature to guarantee
    /// the final response matches this JSON schema.
    pub fn response_schema(mut self, schema: serde_json::Value) -> Self {
        self.response_schema = Some(schema);
        self
    }

    /// Return tool output directly instead of LLM's final answer
    ///
    /// When enabled, the agent will return the last successful tool output directly,
    /// skipping the LLM's summary/wrapping. This is useful when tools already return
    /// perfectly structured JSON and you want to avoid the LLM adding explanations.
    pub fn return_tool_output(mut self, enabled: bool) -> Self {
        self.return_tool_output = enabled;
        self
    }

    /// Build the agent configuration
    ///
    /// Returns a tuple suitable for use with `supervisor::orchestrate_custom_agents`
    /// or for creating SpecializedAgent instances.
    ///
    /// Format: (name, description, system_prompt, tools, response_schema)
    ///
    /// Note: return_tool_output is automatically enabled when response_schema is set
    pub fn build(
        self,
    ) -> (
        String,
        String,
        String,
        Vec<Arc<dyn Tool>>,
        Option<serde_json::Value>,
        bool,
    ) {
        let description = self
            .description
            .unwrap_or_else(|| format!("Specialized agent: {}", self.name));

        let system_prompt = self.system_prompt.unwrap_or_else(|| {
            format!(
                "You are a specialized agent named {}. Use your available tools to complete tasks.",
                self.name
            )
        });

        (
            self.name,
            description,
            system_prompt,
            self.tools,
            self.response_schema,
            self.return_tool_output,
        )
    }

    /// Get the agent name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the number of tools registered
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

/// Collection of agent builders for managing multiple agents
///
/// Provides utility methods for working with multiple agents
/// as a group, making it easier to pass to supervisor APIs.
pub struct AgentCollection {
    agents: Vec<(
        String,
        String,
        String,
        Vec<Arc<dyn Tool>>,
        Option<serde_json::Value>,
        bool,
    )>,
}

impl AgentCollection {
    /// Create an empty agent collection
    pub fn new() -> Self {
        Self { agents: Vec::new() }
    }

    /// Add an agent from a builder
    pub fn add(mut self, builder: AgentBuilder) -> Self {
        self.agents.push(builder.build());
        self
    }

    /// Add a pre-built agent configuration
    pub fn add_config(
        mut self,
        config: (
            String,
            String,
            String,
            Vec<Arc<dyn Tool>>,
            Option<serde_json::Value>,
            bool,
        ),
    ) -> Self {
        self.agents.push(config);
        self
    }

    /// Build into a vector of agent configurations
    pub fn build(
        self,
    ) -> Vec<(
        String,
        String,
        String,
        Vec<Arc<dyn Tool>>,
        Option<serde_json::Value>,
        bool,
    )> {
        self.agents
    }

    /// Get the number of agents in the collection
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// List all agent names and descriptions
    pub fn list_agents(&self) -> Vec<(&str, &str)> {
        self.agents
            .iter()
            .map(|(name, desc, _, _, _, _)| (name.as_str(), desc.as_str()))
            .collect()
    }
}

impl Default for AgentCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for easily registering multiple tools
///
/// # Example
/// ```ignore
/// let tools = tools![
///     AddItemTool::new(),
///     SearchItemsTool::new(),
///     CountItemsTool::new()
/// ];
/// ```
#[macro_export]
macro_rules! tools {
    ($($tool:expr),* $(,)?) => {
        vec![$($tool),*]
    };
}

/// Macro for creating an agent builder with inline configuration
///
/// # Example
/// ```ignore
/// let agent = agent! {
///     name: "data_agent",
///     description: "Manages inventory data",
///     system_prompt: "You are a data management specialist",
///     tools: [
///         AddItemTool::new(),
///         SearchItemsTool::new(),
///     ]
/// };
/// ```
#[macro_export]
macro_rules! agent {
    (
        name: $name:expr,
        description: $desc:expr,
        system_prompt: $prompt:expr,
        tools: [$($tool:expr),* $(,)?]
    ) => {
        $crate::AgentBuilder::new($name)
            .description($desc)
            .system_prompt($prompt)
            $(.tool($tool))*
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{ToolMetadata, ToolResult};
    use async_trait::async_trait;
    use serde_json::Value;

    struct DummyTool;

    #[async_trait]
    impl Tool for DummyTool {
        fn metadata(&self) -> ToolMetadata {
            ToolMetadata {
                name: "dummy".to_string(),
                description: "A dummy tool".to_string(),
                parameters: vec![],
            }
        }

        async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult::success("dummy"))
        }
    }

    #[test]
    fn test_agent_builder_basic() {
        let builder = AgentBuilder::new("test_agent")
            .description("Test agent")
            .system_prompt("Test prompt")
            .tool(DummyTool);

        assert_eq!(builder.name(), "test_agent");
        assert_eq!(builder.tool_count(), 1);

        let (name, desc, prompt, tools, schema, return_tool_output) = builder.build();
        assert_eq!(name, "test_agent");
        assert_eq!(desc, "Test agent");
        assert_eq!(prompt, "Test prompt");
        assert_eq!(tools.len(), 1);
        assert!(schema.is_none());
        assert_eq!(return_tool_output, false);
    }

    #[test]
    fn test_agent_builder_defaults() {
        let builder = AgentBuilder::new("test_agent").tool(DummyTool);

        let (name, desc, prompt, _tools, _schema, _return_tool_output) = builder.build();
        assert_eq!(name, "test_agent");
        assert!(desc.contains("test_agent"));
        assert!(prompt.contains("test_agent"));
    }

    #[test]
    fn test_agent_collection() {
        let agent1 = AgentBuilder::new("agent1").tool(DummyTool);
        let agent2 = AgentBuilder::new("agent2").tool(DummyTool);

        let collection = AgentCollection::new().add(agent1).add(agent2);

        assert_eq!(collection.len(), 2);
        assert_eq!(collection.is_empty(), false);

        let agents = collection.build();
        assert_eq!(agents.len(), 2);
    }

    #[test]
    fn test_agent_collection_list() {
        let agent1 = AgentBuilder::new("agent1")
            .description("First agent")
            .tool(DummyTool);
        let agent2 = AgentBuilder::new("agent2")
            .description("Second agent")
            .tool(DummyTool);

        let collection = AgentCollection::new().add(agent1).add(agent2);

        let list = collection.list_agents();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].0, "agent1");
        assert_eq!(list[1].0, "agent2");
    }
}
