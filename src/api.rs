//! Simple async API facade
//!
//! This module provides a simple, easy-to-use async interface
//! that hides the complexity of the actor system underneath.

use crate::actors::messages::*;
use crate::System;
use anyhow::Result;
use tokio::sync::oneshot;

/// Simple chat function - just send a prompt and get a response
///
/// # Example
/// ```no_run
/// use actorus::{init, chat};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     init().await?;
///     let response = chat("What is Rust?").await?;
///     println!("{}", response);
///     Ok(())
/// }
/// ```
pub async fn chat(prompt: impl Into<String>) -> Result<String> {
    chat_with_system(prompt, None).await
}

/// Chat with a custom system prompt
pub async fn chat_with_system(
    prompt: impl Into<String>,
    system_prompt: Option<String>,
) -> Result<String> {
    let global_system = System::global();

    let mut messages = vec![];

    if let Some(sys) = system_prompt {
        messages.push(ChatMessageData {
            role: "system".to_string(),
            content: sys,
        });
    }

    messages.push(ChatMessageData {
        role: "user".to_string(),
        content: prompt.into(),
    });

    let (tx, rx) = oneshot::channel();
    let request = ChatRequest {
        messages,
        stream: false,
        response: tx,
    };

    global_system
        .router
        .send_message(RoutingMessage::LLM(LLMMessage::Chat(request)))
        .await?;

    match rx.await? {
        ChatResponse::Complete(content) => Ok(content),
        ChatResponse::Error(e) => Err(anyhow::anyhow!(e)),
        _ => Err(anyhow::anyhow!("Unexpected response")),
    }
}

/// Stream chat responses token by token
pub async fn chat_stream(
    prompt: impl Into<String>,
    mut callback: impl FnMut(String),
) -> Result<String> {
    let system = System::global();

    let messages = vec![ChatMessageData {
        role: "user".to_string(),
        content: prompt.into(),
    }];

    let (tx, rx) = oneshot::channel();
    let request = ChatRequest {
        messages,
        stream: true,
        response: tx,
    };

    system
        .router
        .send_message(RoutingMessage::LLM(LLMMessage::Chat(request)))
        .await?;

    match rx.await? {
        ChatResponse::StreamTokens(mut stream_rx) => {
            let mut full_response = String::new();
            while let Some(token) = stream_rx.recv().await {
                callback(token.clone());
                full_response.push_str(&token);
            }
            Ok(full_response)
        }
        ChatResponse::Complete(content) => Ok(content),
        ChatResponse::Error(e) => Err(anyhow::anyhow!(e)),
    }
}

/// Conversation builder for multi-turn conversations
#[derive(Debug, Clone)]
pub struct Conversation {
    messages: Vec<ChatMessageData>,
}

impl Conversation {
    pub fn new() -> Self {
        Self { messages: vec![] }
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.messages.push(ChatMessageData {
            role: "system".to_string(),
            content: system.into(),
        });
        self
    }

    pub fn user(mut self, message: impl Into<String>) -> Self {
        self.messages.push(ChatMessageData {
            role: "user".to_string(),
            content: message.into(),
        });
        self
    }

    pub fn assistant(mut self, message: impl Into<String>) -> Self {
        self.messages.push(ChatMessageData {
            role: "assistant".to_string(),
            content: message.into(),
        });
        self
    }

    pub async fn send(self) -> Result<String> {
        let system = System::global();

        let (tx, rx) = oneshot::channel();
        let request = ChatRequest {
            messages: self.messages,
            stream: false,
            response: tx,
        };

        system
            .router
            .send_message(RoutingMessage::LLM(LLMMessage::Chat(request)))
            .await?;

        match rx.await? {
            ChatResponse::Complete(content) => Ok(content),
            ChatResponse::Error(e) => Err(anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP (Model Context Protocol) API
pub mod mcp {
    use super::*;

    pub async fn list_tools(server_command: &str, server_args: Vec<String>) -> Result<Vec<String>> {
        let system = System::global();

        let (tx, rx) = oneshot::channel();
        let request = MCPListTools {
            server_command: server_command.to_string(),
            server_args,
            response: tx,
        };

        system
            .router
            .send_message(RoutingMessage::MCP(MCPMessage::ListTools(request)))
            .await?;

        match rx.await? {
            MCPResponse::Tools(tools) => Ok(tools),
            MCPResponse::Error(e) => Err(anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    pub async fn call_tool(
        server_command: &str,
        server_args: Vec<String>,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<String> {
        let system = System::global();

        let (tx, rx) = oneshot::channel();
        let request = MCPToolCall {
            server_command: server_command.to_string(),
            server_args,
            tool_name: tool_name.to_string(),
            arguments,
            response: tx,
        };

        system
            .router
            .send_message(RoutingMessage::MCP(MCPMessage::CallTool(request)))
            .await?;

        match rx.await? {
            MCPResponse::Content(content) => Ok(content),
            MCPResponse::Error(e) => Err(anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }
}

/// Batch processing utilities
pub mod batch {
    use super::*;
    use futures::stream::{self, StreamExt};

    pub async fn process_prompts(prompts: Vec<String>, concurrency: usize) -> Vec<Result<String>> {
        stream::iter(prompts)
            .map(|prompt| async move { chat(prompt).await })
            .buffer_unordered(concurrency)
            .collect()
            .await
    }

    pub async fn process_with_context(
        prompts: Vec<(String, String)>, // (prompt, context)
        concurrency: usize,
    ) -> Vec<Result<String>> {
        stream::iter(prompts)
            .map(|(prompt, context)| async move { chat_with_system(prompt, Some(context)).await })
            .buffer_unordered(concurrency)
            .collect()
            .await
    }
}

/// Agent API - Autonomous agent with tool execution capabilities
pub mod agent {
    use super::*;
    use crate::actors::messages::{AgentMessage, AgentResponse, AgentTask, AgentStep};
    use std::sync::Arc;

    /// Run an autonomous agent task
    ///
    /// The agent will use available tools to accomplish the task autonomously.
    ///
    /// # Example
    /// ```no_run
    /// use actorus::{init, agent};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     init().await?;
    ///     let result = agent::run_task("Create a file called hello.txt with 'Hello, World!'").await?;
    ///     println!("Agent result: {}", result.result);
    ///     Ok(())
    /// }
    /// ```
    pub async fn run_task(task: impl Into<String>) -> Result<AgentResult> {
        run_task_with_iterations(task, 10).await
    }

    /// Run an autonomous agent task with custom max iterations
    pub async fn run_task_with_iterations(
        task: impl Into<String>,
        max_iterations: usize,
    ) -> Result<AgentResult> {
        let system = System::global();
        let task_desc = task.into();

        let (tx, rx) = oneshot::channel();
        let agent_task = AgentTask {
            task_description: task_desc.clone(),
            max_iterations: Some(max_iterations),
            response: tx,
        };

        system
            .router
            .send_message(RoutingMessage::Agent(AgentMessage::RunTask(agent_task)))
            .await?;

        let response = rx.await?;

        Ok(AgentResult::from_response(response))
    }

    /// Run an autonomous agent task with custom tools
    ///
    /// Creates a specialized agent with your custom tools and runs the task.
    /// This allows you to use domain-specific tools with the LLM agent.
    ///
    /// # Example
    /// ```no_run
    /// use actorus::{init, agent, tool_fn, tools::Tool};
    /// use std::sync::Arc;
    /// use anyhow::Result;
    ///
    /// #[tool_fn(name = "greet", description = "Greet someone")]
    /// async fn greet(name: String) -> Result<String> {
    ///     Ok(format!("Hello, {}!", name))
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     init().await?;
    ///
    ///     let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(GreetTool::new())];
    ///
    ///     let result = agent::run_task_with_tools(
    ///         tools,
    ///         "Greet Alice using the greet tool"
    ///     ).await?;
    ///
    ///     println!("Result: {}", result.result);
    ///     Ok(())
    /// }
    /// ```
    pub async fn run_task_with_tools(
        tools: Vec<Arc<dyn crate::tools::Tool>>,
        task: impl Into<String>,
    ) -> Result<AgentResult> {
        run_task_with_tools_and_iterations(tools, task, 10).await
    }

    /// Run a task with custom tools and max iterations
    pub async fn run_task_with_tools_and_iterations(
        tools: Vec<Arc<dyn crate::tools::Tool>>,
        task: impl Into<String>,
        max_iterations: usize,
    ) -> Result<AgentResult> {
        use crate::actors::specialized_agent::{SpecializedAgent, SpecializedAgentConfig};
        use crate::config::Settings;

        let settings = Settings::new()?;
        let api_key = Settings::api_key()?;

        let config = SpecializedAgentConfig {
            name: "custom_tools_agent".to_string(),
            description: "Agent with custom user-provided tools".to_string(),
            system_prompt: "You are an agent with access to custom tools. Use them to complete the user's task.".to_string(),
            tools,
            response_schema: None,
            return_tool_output: false,
        };

        let agent = SpecializedAgent::new(config, settings, api_key);
        let response = agent.execute_task(&task.into(), max_iterations).await;

        Ok(AgentResult::from_response(response))
    }

    /// Stop the agent actor
    ///
    /// Gracefully stops the agent actor. Useful for cleanup or reconfiguration.
    pub async fn stop() -> Result<()> {
        let system = System::global();
        system
            .router
            .send_message(RoutingMessage::Agent(AgentMessage::Stop))
            .await?;
        Ok(())
    }

    /// Result from agent execution
    #[derive(Debug, Clone)]
    pub struct AgentResult {
        pub success: bool,
        pub result: String,
        pub steps: Vec<AgentStepInfo>,
        pub error: Option<String>,
    }

    /// Information about a single agent step
    #[derive(Debug, Clone)]
    pub struct AgentStepInfo {
        pub iteration: usize,
        pub thought: String,
        pub action: Option<String>,
        pub observation: Option<String>,
    }

    impl AgentResult {
        pub(crate) fn from_response(response: AgentResponse) -> Self {
            match response {
                AgentResponse::Success { result, steps, .. } => Self {
                    success: true,
                    result,
                    steps: steps.into_iter().map(AgentStepInfo::from).collect(),
                    error: None,
                },
                AgentResponse::Failure { error, steps, .. } => Self {
                    success: false,
                    result: String::new(),
                    steps: steps.into_iter().map(AgentStepInfo::from).collect(),
                    error: Some(error),
                },
                AgentResponse::Timeout {
                    partial_result,
                    steps,
                    ..
                } => Self {
                    success: false,
                    result: partial_result,
                    steps: steps.into_iter().map(AgentStepInfo::from).collect(),
                    error: Some("Max iterations reached".to_string()),
                },
            }
        }
    }

    impl From<AgentStep> for AgentStepInfo {
        fn from(step: AgentStep) -> Self {
            Self {
                iteration: step.iteration,
                thought: step.thought,
                action: step.action,
                observation: step.observation,
            }
        }
    }
}

/// Router Agent API - Intent classification and routing to specialized agents
pub mod router {
    use super::*;
    use crate::actors::router_agent::RouterAgent;
    use crate::actors::specialized_agents_factory;
    use crate::core::llm::LLMClient;
    use crate::config::Settings;

    pub use crate::actors::messages::{AgentResponse, AgentStep};
    pub use crate::api::agent::{AgentResult, AgentStepInfo};

    /// Route a task to the appropriate specialized agent
    ///
    /// The router uses LLM-based intent classification to determine
    /// which specialized agent (file_ops, shell, web, or general) should
    /// handle the task. This implements the "one-way ticket" pattern where
    /// each query is routed to a single agent.
    ///
    /// # Example
    /// ```no_run
    /// use actorus::{init, router};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     init().await?;
    ///     let result = router::route_task("List all files in the current directory").await?;
    ///     println!("Router result: {}", result.result);
    ///     Ok(())
    /// }
    /// ```
    pub async fn route_task(task: impl Into<String>) -> Result<AgentResult> {
        route_task_with_iterations(task, 10).await
    }

    /// Route a task with custom max iterations per agent
    pub async fn route_task_with_iterations(
        task: impl Into<String>,
        max_iterations: usize,
    ) -> Result<AgentResult> {
        let settings = Settings::new()?;
        let api_key = Settings::api_key()?;

        // Create specialized agents
        let agents = specialized_agents_factory::create_default_agents(settings.clone(), api_key.clone());

        // Create router
        let llm_client = LLMClient::new(api_key, settings);
        let router = RouterAgent::new(agents, llm_client);

        // Route task
        let response = router.route_task(&task.into(), max_iterations).await;

        Ok(AgentResult::from_response(response))
    }

    /// List available specialized agents
    ///
    /// Returns the names of all available specialized agents that the router can use.
    pub fn list_agents() -> Vec<&'static str> {
        vec!["file_ops_agent", "shell_agent", "web_agent", "general_agent"]
    }

    /// Get description of a specialized agent
    ///
    /// Returns a description of what the specified agent can do.
    pub fn agent_info(agent_name: &str) -> Option<&'static str> {
        match agent_name {
            "file_ops_agent" => Some("Handles file system operations including reading and writing files. Use this agent for tasks involving file I/O operations."),
            "shell_agent" => Some("Executes shell commands and system operations. Use this agent for tasks involving command-line operations, directory listings, process management, and system queries."),
            "web_agent" => Some("Handles HTTP requests and web-based operations. Use this agent for tasks involving fetching web content, making API calls, and retrieving online information."),
            "general_agent" => Some("General-purpose agent with access to all tools. Use this agent for tasks that require multiple tool categories or when the task doesn't clearly fit into a specific domain."),
            _ => None,
        }
    }

    /// Route a task to custom specialized agents
    ///
    /// Similar to route_task() but allows you to provide your own specialized agents
    /// with custom tools. The router will use LLM-based intent classification to determine
    /// which of your agents should handle the task.
    ///
    /// # Example
    /// ```no_run
    /// use actorus::{init, router, AgentBuilder, AgentCollection, tool_fn};
    ///
    /// #[tool_fn(name = "greet", description = "Greet someone")]
    /// async fn greet(name: String) -> Result<String> {
    ///     Ok(format!("Hello, {}!", name))
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     init().await?;
    ///
    ///     let greeter_agent = AgentBuilder::new("greeter")
    ///         .description("Greets people")
    ///         .tool(GreetTool::new());
    ///
    ///     let agents = AgentCollection::new().add(greeter_agent);
    ///
    ///     let result = router::route_task_with_custom_agents(
    ///         agents.build(),
    ///         "Greet Alice"
    ///     ).await?;
    ///
    ///     println!("Result: {}", result.result);
    ///     Ok(())
    /// }
    /// ```
    pub async fn route_task_with_custom_agents(
        agent_configs: Vec<(String, String, String, Vec<std::sync::Arc<dyn crate::tools::Tool>>, Option<serde_json::Value>, bool)>,
        task: impl Into<String>,
    ) -> Result<AgentResult> {
        route_task_with_custom_agents_and_iterations(agent_configs, task, 10).await
    }

    /// Route with custom agents and max iterations
    pub async fn route_task_with_custom_agents_and_iterations(
        agent_configs: Vec<(String, String, String, Vec<std::sync::Arc<dyn crate::tools::Tool>>, Option<serde_json::Value>, bool)>,
        task: impl Into<String>,
        max_iterations: usize,
    ) -> Result<AgentResult> {
        use crate::actors::specialized_agent::{SpecializedAgent, SpecializedAgentConfig};
        use crate::actors::router_agent::RouterAgent;
        use crate::core::llm::LLMClient;
        use crate::config::Settings;

        let settings = Settings::new()?;
        let api_key = Settings::api_key()?;

        // Create specialized agents from configs
        let agents: Vec<SpecializedAgent> = agent_configs
            .into_iter()
            .map(|(name, description, system_prompt, tools, response_schema, return_tool_output)| {
                let config = SpecializedAgentConfig {
                    name,
                    description,
                    system_prompt,
                    tools,
                    response_schema,
                    return_tool_output,
                };
                SpecializedAgent::new(config, settings.clone(), api_key.clone())
            })
            .collect();

        // Create router
        let llm_client = LLMClient::new(api_key, settings);
        let router = RouterAgent::new(agents, llm_client);

        // Route task
        let response = router.route_task(&task.into(), max_iterations).await;

        Ok(AgentResult::from_response(response))
    }
}

/// Supervisor Agent API - Multi-agent orchestration for complex tasks
pub mod supervisor {
    use super::*;
    use crate::actors::handoff::HandoffCoordinator;
    use crate::actors::supervisor_agent::SupervisorAgent;
    use crate::actors::specialized_agents_factory;
    use crate::core::llm::LLMClient;
    use crate::config::Settings;
    use std::sync::Arc;

    pub use crate::actors::messages::{AgentResponse, AgentStep};
    pub use crate::api::agent::{AgentResult, AgentStepInfo};

    /// Orchestrate a complex task across multiple specialized agents
    ///
    /// The supervisor decomposes complex multi-step tasks and coordinates
    /// multiple specialized agents to accomplish them. This implements the
    /// "return ticket" pattern where agents can be invoked multiple times
    /// as needed.
    ///
    /// Uses max_orchestration_steps from config (default: 10)
    ///
    /// # Example
    /// ```no_run
    /// use actorus::{init, supervisor};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     init().await?;
    ///     let result = supervisor::orchestrate(
    ///         "List all Rust files, count them, and write the count to result.txt"
    ///     ).await?;
    ///     println!("Supervisor result: {}", result.result);
    ///     Ok(())
    /// }
    /// ```
    pub async fn orchestrate(task: impl Into<String>) -> Result<AgentResult> {
        let settings = Settings::new()?;
        let max_steps = settings.agent.max_orchestration_steps;
        orchestrate_with_steps(task, max_steps).await
    }

    /// Orchestrate with custom max orchestration steps
    pub async fn orchestrate_with_steps(
        task: impl Into<String>,
        max_orchestration_steps: usize,
    ) -> Result<AgentResult> {
        let settings = Settings::new()?;
        let api_key = Settings::api_key()?;

        // Create specialized agents
        let agents = specialized_agents_factory::create_default_agents(settings.clone(), api_key.clone());

        // Create supervisor (ideally would use GPT-4 or higher for better decomposition)
        let llm_client = LLMClient::new(api_key.clone(), settings.clone());
        let supervisor = SupervisorAgent::new(agents, llm_client, settings);

        // Orchestrate task
        let response = supervisor.orchestrate(&task.into(), max_orchestration_steps).await;

        Ok(AgentResult::from_response(response))
    }

    /// Orchestrate a task with custom specialized agents
    ///
    /// Similar to orchestrate() but allows you to provide your own specialized agents
    /// with custom tools. This enables domain-specific multi-agent coordination.
    ///
    /// # Example
    /// ```no_run
    /// use actorus::{init, supervisor, tool_fn, tools::Tool};
    /// use std::sync::Arc;
    ///
    /// #[tool_fn(name = "greet", description = "Greet someone")]
    /// async fn greet(name: String) -> Result<String> {
    ///     Ok(format!("Hello, {}!", name))
    /// }
    ///
    /// // This would require creating specialized agents with custom tools
    /// // See supervisor_with_custom_tools.rs for a working example
    /// ```
    pub async fn orchestrate_custom_agents(
        agent_configs: Vec<(String, String, String, Vec<Arc<dyn crate::tools::Tool>>, Option<serde_json::Value>, bool)>, // (name, description, system_prompt, tools, response_schema, return_tool_output)
        task: impl Into<String>,
    ) -> Result<AgentResult> {
        let settings = Settings::new()?;
        let max_steps = settings.agent.max_orchestration_steps;
        orchestrate_custom_agents_and_steps(agent_configs, task, max_steps).await
    }

    /// Orchestrate with custom agents and max orchestration steps
    pub async fn orchestrate_custom_agents_and_steps(
        agent_configs: Vec<(String, String, String, Vec<Arc<dyn crate::tools::Tool>>, Option<serde_json::Value>, bool)>,
        task: impl Into<String>,
        max_orchestration_steps: usize,
    ) -> Result<AgentResult> {
        use crate::actors::specialized_agent::{SpecializedAgent, SpecializedAgentConfig};
        use crate::actors::supervisor_agent::SupervisorAgent;
        use crate::core::llm::LLMClient;
        use crate::config::Settings;

        let settings = Settings::new()?;
        let api_key = Settings::api_key()?;

        // Create specialized agents from configs
        let agents: Vec<SpecializedAgent> = agent_configs
            .into_iter()
            .map(|(name, description, system_prompt, tools, response_schema, return_tool_output)| {
                let config = SpecializedAgentConfig {
                    name,
                    description,
                    system_prompt,
                    tools,
                    response_schema,
                    return_tool_output,
                };
                SpecializedAgent::new(config, settings.clone(), api_key.clone())
            })
            .collect();

        // Create supervisor
        let llm_client = LLMClient::new(api_key.clone(), settings.clone());
        let supervisor = SupervisorAgent::new(agents, llm_client, settings);

        // Orchestrate task
        let response = supervisor.orchestrate(&task.into(), max_orchestration_steps).await;

        Ok(AgentResult::from_response(response))
    }

    /// List available specialized agents
    ///
    /// Returns the names of all available specialized agents that the supervisor can coordinate.
    pub fn list_agents() -> Vec<&'static str> {
        vec!["file_ops_agent", "shell_agent", "web_agent", "general_agent"]
    }

    /// Orchestrate with handoff validation enabled
    ///
    /// This variant enables quality gates between agent outputs. Each agent's output
    /// is validated against contracts before being passed to the next agent.
    ///
    /// # Example
    /// ```no_run
    /// use actorus::{init, supervisor};
    /// use actorus::actors::handoff::HandoffCoordinator;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     init().await?;
    ///
    ///     // Setup validation contracts
    ///     let mut coordinator = HandoffCoordinator::new();
    ///     // Register contracts here...
    ///
    ///     let result = supervisor::orchestrate_with_validation(
    ///         coordinator,
    ///         "Analyze sales data and generate report"
    ///     ).await?;
    ///
    ///     println!("Result: {}", result.result);
    ///     Ok(())
    /// }
    /// ```
    pub async fn orchestrate_with_validation(
        coordinator: HandoffCoordinator,
        task: impl Into<String>,
    ) -> Result<AgentResult> {
        let settings = Settings::new()?;
        let max_steps = settings.agent.max_orchestration_steps;
        orchestrate_with_validation_and_steps(coordinator, task, max_steps).await
    }

    /// Orchestrate with validation and custom max orchestration steps
    pub async fn orchestrate_with_validation_and_steps(
        coordinator: HandoffCoordinator,
        task: impl Into<String>,
        max_orchestration_steps: usize,
    ) -> Result<AgentResult> {
        let settings = Settings::new()?;
        let api_key = Settings::api_key()?;

        // Create specialized agents
        let agents = specialized_agents_factory::create_default_agents(settings.clone(), api_key.clone());

        // Create supervisor with validation
        let llm_client = LLMClient::new(api_key.clone(), settings.clone());
        let supervisor = SupervisorAgent::new(agents, llm_client, settings)
            .with_handoff_validation(coordinator);

        // Orchestrate task
        let response = supervisor.orchestrate(&task.into(), max_orchestration_steps).await;

        Ok(AgentResult::from_response(response))
    }

    /// Orchestrate custom agents with handoff validation
    ///
    /// Combines custom agents with validation quality gates for maximum flexibility
    /// and reliability.
    ///
    /// # Example
    /// ```no_run
    /// use actorus::{init, supervisor, AgentBuilder, AgentCollection};
    /// use actorus::actors::handoff::{HandoffCoordinator, HandoffContract};
    /// use actorus::actors::messages::OutputSchema;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     init().await?;
    ///
    ///     // Build custom agents
    ///     let data_agent = AgentBuilder::new("data_agent")
    ///         .description("Fetches data");
    ///     let agents = AgentCollection::new().add(data_agent);
    ///
    ///     // Setup validation
    ///     let mut coordinator = HandoffCoordinator::new();
    ///     // Register contracts...
    ///
    ///     let result = supervisor::orchestrate_custom_agents_with_validation(
    ///         coordinator,
    ///         agents.build(),
    ///         "Fetch and analyze data"
    ///     ).await?;
    ///
    ///     println!("Result: {}", result.result);
    ///     Ok(())
    /// }
    /// ```
    pub async fn orchestrate_custom_agents_with_validation(
        coordinator: HandoffCoordinator,
        agent_configs: Vec<(String, String, String, Vec<Arc<dyn crate::tools::Tool>>, Option<serde_json::Value>, bool)>,
        task: impl Into<String>,
    ) -> Result<AgentResult> {
        let settings = Settings::new()?;
        let max_steps = settings.agent.max_orchestration_steps;
        orchestrate_custom_agents_with_validation_and_steps(coordinator, agent_configs, task, max_steps).await
    }

    /// Orchestrate custom agents with validation and custom max orchestration steps
    pub async fn orchestrate_custom_agents_with_validation_and_steps(
        coordinator: HandoffCoordinator,
        agent_configs: Vec<(String, String, String, Vec<Arc<dyn crate::tools::Tool>>, Option<serde_json::Value>, bool)>,
        task: impl Into<String>,
        max_orchestration_steps: usize,
    ) -> Result<AgentResult> {
        use crate::actors::specialized_agent::{SpecializedAgent, SpecializedAgentConfig};
        use crate::actors::supervisor_agent::SupervisorAgent;
        use crate::core::llm::LLMClient;
        use crate::config::Settings;

        let settings = Settings::new()?;
        let api_key = Settings::api_key()?;

        // Create specialized agents from configs
        let agents: Vec<SpecializedAgent> = agent_configs
            .into_iter()
            .map(|(name, description, system_prompt, tools, response_schema, return_tool_output)| {
                let config = SpecializedAgentConfig {
                    name,
                    description,
                    system_prompt,
                    tools,
                    response_schema,
                    return_tool_output,
                };
                SpecializedAgent::new(config, settings.clone(), api_key.clone())
            })
            .collect();

        // Create supervisor with validation
        let llm_client = LLMClient::new(api_key.clone(), settings.clone());
        let supervisor = SupervisorAgent::new(agents, llm_client, settings)
            .with_handoff_validation(coordinator);

        // Orchestrate task
        let response = supervisor.orchestrate(&task.into(), max_orchestration_steps).await;

        Ok(AgentResult::from_response(response))
    }
}

/// Session API - Persistent multi-turn conversations with agents
pub mod session {
    use super::*;
    use crate::actors::agent_session::AgentSession;
    use crate::storage::{ConversationStorage, memory::InMemoryStorage, filesystem::FileSystemStorage};
    use crate::config::Settings;
    use std::sync::Arc;
    use std::path::PathBuf;

    pub use crate::api::agent::{AgentResult, AgentStepInfo};

    /// Storage backend type for sessions
    pub enum StorageType {
        /// In-memory storage (lost on process termination)
        Memory,
        /// File system storage (persists to disk)
        FileSystem(PathBuf),
    }

    /// Create a new agent session with persistent conversation history
    ///
    /// Sessions maintain conversation context across multiple tasks, allowing
    /// for natural multi-turn interactions where the agent remembers previous
    /// context.
    ///
    /// # Example - In-memory session (ephemeral)
    /// ```no_run
    /// use actorus::api::session::{self, StorageType};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let mut session = session::create_session(
    ///         "user-123",
    ///         StorageType::Memory,
    ///     ).await?;
    ///
    ///     // First task
    ///     let result = session.send_message("What files are in /tmp?").await?;
    ///     println!("{}", result.result);
    ///
    ///     // Second task remembers context from first
    ///     let result = session.send_message("Delete the .txt files you just found").await?;
    ///     println!("{}", result.result);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Example - Persistent session
    /// ```no_run
    /// use actorus::api::session::{self, StorageType};
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let mut session = session::create_session(
    ///         "user-123",
    ///         StorageType::FileSystem(PathBuf::from("./sessions")),
    ///     ).await?;
    ///
    ///     let result = session.send_message("Remember: my favorite color is blue").await?;
    ///     // Session persists to disk automatically
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_session(
        session_id: impl Into<String>,
        storage_type: StorageType,
    ) -> Result<Session> {
        let settings = Settings::new()?;
        let api_key = Settings::api_key()?;

        let storage: Arc<dyn ConversationStorage> = match storage_type {
            StorageType::Memory => Arc::new(InMemoryStorage::new()),
            StorageType::FileSystem(path) => Arc::new(FileSystemStorage::new(path).await?),
        };

        let inner = AgentSession::new(session_id, storage, settings, api_key).await?;

        Ok(Session { inner })
    }

    /// Session handle for multi-turn conversations
    pub struct Session {
        inner: AgentSession,
    }

    impl Session {
        /// Send a message to the agent and get a response
        ///
        /// The conversation history is automatically maintained and persisted.
        pub async fn send_message(&mut self, message: &str) -> Result<AgentResult> {
            self.send_message_with_iterations(message, 10).await
        }

        /// Send a message with custom max iterations
        pub async fn send_message_with_iterations(
            &mut self,
            message: &str,
            max_iterations: usize,
        ) -> Result<AgentResult> {
            // Temporarily set max_iterations
            let old_max_iterations = self.inner.max_iterations();
            self.inner.set_max_iterations(max_iterations);

            let session_response = self.inner.send_message(message).await?;

            // Restore old max_iterations
            self.inner.set_max_iterations(old_max_iterations);

            // Convert SessionResponse to AgentResult
            Ok(AgentResult {
                success: session_response.completed,
                result: session_response.message.clone(),
                steps: session_response.steps.iter().enumerate().map(|(i, step)| AgentStepInfo {
                    iteration: i,
                    thought: step.thought.clone(),
                    action: step.action.clone(),
                    observation: step.observation.clone(),
                }).collect(),
                error: if session_response.completed { None } else { Some(session_response.message) },
            })
        }

        /// Clear conversation history for this session
        pub async fn clear_history(&mut self) -> Result<()> {
            self.inner.clear_history().await
        }

        /// Get the session ID
        pub fn session_id(&self) -> &str {
            self.inner.session_id()
        }

        /// Get the number of messages in the conversation history
        pub fn message_count(&self) -> usize {
            self.inner.history().len()
        }
    }
}
