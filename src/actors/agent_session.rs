//! Agent Session - Stateful Agent with Persistent Conversation
//!
//! Information Hiding:
//! - Storage backend hidden from session users
//! - Conversation history management internalized
//! - Session lifecycle management hidden

use crate::config::Settings;
use crate::core::llm::{ChatMessage, LLMClient};
use crate::storage::ConversationStorage;
use crate::tools::{executor::ToolExecutor, registry::ToolRegistry, ToolConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Agent session with persistent conversation history
pub struct AgentSession {
    session_id: String,
    conversation_history: Vec<ChatMessage>,
    llm_client: LLMClient,
    tool_registry: Arc<ToolRegistry>,
    tool_executor: ToolExecutor,
    storage: Arc<dyn ConversationStorage>,
    pub(crate) max_iterations: usize,
}

/// Decision structure returned by LLM
#[derive(Debug, Deserialize, Serialize)]
struct AgentDecision {
    thought: String,
    action: Option<AgentAction>,
    is_final: bool,
    final_answer: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AgentAction {
    tool: String,
    input: Value,
}

/// Step taken by agent during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStep {
    pub thought: String,
    pub action: Option<String>,
    pub observation: Option<String>,
}

impl AgentSession {
    /// Create a new agent session
    pub async fn new(
        session_id: impl Into<String>,
        storage: Arc<dyn ConversationStorage>,
        settings: Settings,
        api_key: String,
    ) -> Result<Self> {
        let session_id = session_id.into();

        // Try to load existing conversation
        let conversation_history = storage
            .load(&session_id)
            .await
            .unwrap_or_else(|_| Vec::new());

        let llm_client = LLMClient::new(api_key, settings.clone());
        let tool_registry = Arc::new(ToolRegistry::with_defaults());
        let tool_executor = ToolExecutor::new(ToolConfig::default());

        Ok(Self {
            session_id,
            conversation_history,
            llm_client,
            tool_registry,
            tool_executor,
            storage,
            max_iterations: settings.agent.max_iterations,
        })
    }

    /// Set maximum iterations (mutable version)
    pub fn set_max_iterations(&mut self, max_iterations: usize) {
        self.max_iterations = max_iterations;
    }

    /// Get current max iterations setting
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    /// Send a message and get response (maintains conversation context)
    pub async fn send_message(&mut self, message: &str) -> Result<SessionResponse> {
        // If this is the first message, add system prompt
        if self.conversation_history.is_empty() {
            let system_prompt = format!(
                "You are an autonomous agent that can use tools OR respond directly to accomplish tasks.\n\n\
                 Available Tools:\n{}\n\n\
                 IMPORTANT: You MUST respond in this EXACT JSON format:\n\
                 {{\n  \
                   \"thought\": \"your reasoning about what to do next\",\n  \
                   \"action\": {{\"tool\": \"tool_name\", \"input\": {{\"param\": \"value\"}}}},\n  \
                   \"is_final\": false,\n  \
                   \"final_answer\": null\n\
                 }}\n\n\
                 DECISION GUIDELINES:\n\
                 1. For conversational messages (greetings, questions about context, general chat):\n\
                    - Set \"is_final\": true immediately\n\
                    - Set \"action\": null (no tool needed)\n\
                    - Provide your answer in \"final_answer\"\n\
                 2. For tasks requiring tools (file operations, shell commands, web requests):\n\
                    - Choose appropriate tool\n\
                    - Execute action\n\
                    - After getting the observation, set \"is_final\": true with \"final_answer\"\n\n\
                 EXAMPLES:\n\
                 User: \"hi\" → {{\"thought\": \"greeting\", \"action\": null, \"is_final\": true, \"final_answer\": \"Hello! How can I help you?\"}}\n\
                 User: \"list files\" → {{\"thought\": \"need shell tool\", \"action\": {{\"tool\": \"execute_shell\", \"input\": {{\"command\": \"ls\"}}}}, \"is_final\": false, \"final_answer\": null}}\n\n\
                 Always respond with valid JSON only. No extra text.",
                self.tool_registry.tools_description()
            );

            self.conversation_history.push(ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            });
        }

        // Add user message
        self.conversation_history.push(ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        // Execute ReAct loop with existing conversation context
        let response = self.execute_react_loop().await?;

        // Persist updated history
        self.storage
            .save(&self.session_id, &self.conversation_history)
            .await?;

        Ok(response)
    }

    /// Clear conversation history
    pub async fn clear_history(&mut self) -> Result<()> {
        self.conversation_history.clear();
        self.storage.delete(&self.session_id).await?;
        Ok(())
    }

    /// Get conversation history
    pub fn history(&self) -> &[ChatMessage] {
        &self.conversation_history
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Execute ReAct loop with existing conversation history
    async fn execute_react_loop(&mut self) -> Result<SessionResponse> {
        let mut steps = Vec::new();

        for iteration in 0..self.max_iterations {
            tracing::debug!(
                "[Session {}] Iteration {}/{}",
                self.session_id,
                iteration + 1,
                self.max_iterations
            );

            // Think: Ask LLM for next action
            let decision = self.think().await?;

            tracing::debug!(
                "[Session {}] Thought: {}",
                self.session_id,
                decision.thought
            );

            // Check if task is complete
            if decision.is_final {
                let final_answer = decision
                    .final_answer
                    .unwrap_or_else(|| "Task completed".to_string());

                steps.push(SessionStep {
                    thought: decision.thought,
                    action: None,
                    observation: Some(final_answer.clone()),
                });

                return Ok(SessionResponse {
                    message: final_answer,
                    steps,
                    completed: true,
                });
            }

            // Act: Execute the tool
            if let Some(action) = decision.action {
                tracing::info!(
                    "[Session {}] Executing tool: {}",
                    self.session_id,
                    action.tool
                );

                let tool = match self.tool_registry.get(&action.tool) {
                    Some(t) => t,
                    None => {
                        let error_msg = format!("Tool '{}' not found", action.tool);
                        self.conversation_history.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: format!("Error: {}", error_msg),
                        });

                        steps.push(SessionStep {
                            thought: decision.thought,
                            action: Some(action.tool.clone()),
                            observation: Some(error_msg.clone()),
                        });

                        return Ok(SessionResponse {
                            message: error_msg,
                            steps,
                            completed: false,
                        });
                    }
                };

                // Observe: Get tool result
                let tool_result = self
                    .tool_executor
                    .execute(tool, action.input.clone())
                    .await?;

                let observation = if tool_result.success {
                    tool_result.output.clone()
                } else {
                    format!("Tool failed: {}", tool_result.error.unwrap_or_default())
                };

                tracing::debug!("[Session {}] Observation: {}", self.session_id, observation);

                // Add agent's action to conversation history
                self.conversation_history.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: serde_json::to_string(&AgentDecision {
                        thought: decision.thought.clone(),
                        action: Some(action.clone()),
                        is_final: false,
                        final_answer: None,
                    })
                    .unwrap_or_else(|_| format!("Action: {}", action.tool)),
                });

                // Add observation to conversation
                self.conversation_history.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!(
                        "Observation: {}\n\nDoes this observation contain the answer? \
                         If yes, set is_final=true and provide final_answer. \
                         If no, what is the next action needed?",
                        observation
                    ),
                });

                steps.push(SessionStep {
                    thought: decision.thought,
                    action: Some(action.tool.clone()),
                    observation: Some(observation),
                });
            } else {
                // No action but also not marked as final - this is likely a conversational response
                // Treat the thought as the final answer
                if !decision.thought.is_empty() {
                    tracing::info!(
                        "[Session {}] No action needed, treating as direct response",
                        self.session_id
                    );

                    let final_answer = decision.thought.clone();

                    // Add assistant's response to conversation history
                    self.conversation_history.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: final_answer.clone(),
                    });

                    steps.push(SessionStep {
                        thought: decision.thought,
                        action: None,
                        observation: Some(final_answer.clone()),
                    });

                    return Ok(SessionResponse {
                        message: final_answer,
                        steps,
                        completed: true,
                    });
                }

                let error_msg = "No action specified and no response provided".to_string();
                steps.push(SessionStep {
                    thought: decision.thought,
                    action: None,
                    observation: Some(error_msg.clone()),
                });

                return Ok(SessionResponse {
                    message: error_msg,
                    steps,
                    completed: false,
                });
            }
        }

        // Max iterations reached
        Ok(SessionResponse {
            message: "Max iterations reached without completing task".to_string(),
            steps,
            completed: false,
        })
    }

    /// Think step - Ask LLM to reason about next action
    async fn think(&self) -> Result<AgentDecision> {
        let response = self
            .llm_client
            .chat(self.conversation_history.clone())
            .await?;

        // Try to parse JSON response
        match serde_json::from_str::<AgentDecision>(&response) {
            Ok(decision) => Ok(decision),
            Err(e) => {
                tracing::warn!(
                    "[Session {}] Failed to parse decision as JSON: {}",
                    self.session_id,
                    e
                );

                // Try to find JSON in the response
                if let Some(start) = response.find('{') {
                    if let Some(end) = response.rfind('}') {
                        let json_str = &response[start..=end];
                        if let Ok(decision) = serde_json::from_str::<AgentDecision>(json_str) {
                            return Ok(decision);
                        }
                    }
                }

                // If all parsing fails, treat response as a direct conversational answer
                // This happens when LLM responds naturally instead of following JSON format
                tracing::info!(
                    "[Session {}] Treating non-JSON response as direct answer",
                    self.session_id
                );
                Ok(AgentDecision {
                    thought: response.clone(),
                    action: None,
                    is_final: true,
                    final_answer: Some(response),
                })
            }
        }
    }
}

/// Response from a session message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub message: String,
    pub steps: Vec<SessionStep>,
    pub completed: bool,
}
