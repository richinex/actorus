//! Specialized Agent - Domain-specific ReAct agent
//!
//! Information Hiding:
//! - Hides specific tool sets from coordinator
//! - Encapsulates domain-specific prompts
//! - Internal ReAct loop implementation hidden
//! - Exposes simple task execution interface

use crate::actors::messages::{
    AgentResponse, AgentStep, CompletionStatus, OutputMetadata, ToolCallMetadata,
};
use crate::config::Settings;
use crate::core::llm::{ChatMessage, LLMClient};
use crate::tools::{executor::ToolExecutor, registry::ToolRegistry, Tool, ToolConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

/// Configuration for a specialized agent
#[derive(Clone)]
pub struct SpecializedAgentConfig {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub tools: Vec<Arc<dyn Tool>>,
    pub response_schema: Option<serde_json::Value>,
    /// If true, return the last successful tool output directly instead of the agent's final_answer
    /// This is useful when tools return structured JSON and you want to skip LLM wrapping
    pub return_tool_output: bool,
}

impl std::fmt::Debug for SpecializedAgentConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpecializedAgentConfig")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("system_prompt", &self.system_prompt)
            .field("tools_count", &self.tools.len())
            .field("has_response_schema", &self.response_schema.is_some())
            .field("return_tool_output", &self.return_tool_output)
            .finish()
    }
}

/// Decision structure returned by specialized agent's LLM
#[derive(Debug, Clone, Deserialize, Serialize)]
struct AgentDecision {
    thought: String,
    action: Option<AgentAction>,
    is_final: bool,
    #[serde(deserialize_with = "deserialize_final_answer")]
    final_answer: Option<String>,
}

/// Custom deserializer that accepts either a string or JSON value
fn deserialize_final_answer<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let value: Option<Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(other) => {
            // Convert any JSON value to a pretty-printed string
            Ok(Some(
                serde_json::to_string_pretty(&other).map_err(Error::custom)?,
            ))
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AgentAction {
    tool: String,
    input: Value,
}

/// Specialized agent that focuses on a specific domain
pub struct SpecializedAgent {
    config: SpecializedAgentConfig,
    llm_client: LLMClient,
    tool_registry: ToolRegistry,
    tool_executor: ToolExecutor,
}

impl SpecializedAgent {
    pub fn new(config: SpecializedAgentConfig, settings: Settings, api_key: String) -> Self {
        let mut tool_registry = ToolRegistry::new();
        for tool in &config.tools {
            tool_registry.register(Arc::clone(tool));
        }

        Self {
            config,
            llm_client: LLMClient::new(api_key, settings),
            tool_registry,
            tool_executor: ToolExecutor::new(ToolConfig::default()),
        }
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn description(&self) -> &str {
        &self.config.description
    }

    /// Execute a task using this specialized agent
    pub async fn execute_task(&self, task: &str, max_iterations: usize) -> AgentResponse {
        self.execute_task_with_context(task, None, max_iterations)
            .await
    }

    /// Execute a task with additional context data
    ///
    /// Context data is structured information that can be referenced by the agent.
    /// This is useful for multi-agent pipelines where one agent's output becomes
    /// another agent's input.
    ///
    /// # Example
    /// ```ignore
    /// let context = serde_json::json!({
    ///     "previous_results": {...},
    ///     "database_output": {...}
    /// });
    /// agent.execute_task_with_context("Analyze the data", Some(context), 10).await
    /// ```
    pub async fn execute_task_with_context(
        &self,
        task: &str,
        context: Option<Value>,
        max_iterations: usize,
    ) -> AgentResponse {
        let start_time = Instant::now();
        let mut steps = Vec::new();
        let mut conversation_history = Vec::new();
        let mut tool_calls = Vec::new();
        let mut last_tool_output: Option<String> = None;

        // Build system prompt with available tools and context
        let context_section = if let Some(ctx) = &context {
            format!(
                "\n\nCONTEXT DATA (use this in your tool calls):\n```json\n{}\n```\n\
                     The context contains structured data from previous steps. \
                     You can reference fields from this data when calling tools.",
                serde_json::to_string_pretty(ctx).unwrap_or_else(|_| "{}".to_string())
            )
        } else {
            String::new()
        };

        let system_prompt = format!(
            "{}\n\nAvailable Tools:\n{}{}\n\n\
             IMPORTANT: You have a maximum of {} iterations to complete this task.\n\
             You MUST respond in this EXACT JSON format:\n\
             {{\n  \
               \"thought\": \"your reasoning about what to do next\",\n  \
               \"action\": {{\"tool\": \"tool_name\", \"input\": {{\"param\": \"value\"}}}},\n  \
               \"is_final\": false,\n  \
               \"final_answer\": null\n\
             }}\n\n\
             When the task is COMPLETE:\n\
             - Set \"is_final\": true\n\
             - Set \"action\": null\n\
             - Provide a clear \"final_answer\" summarizing what you accomplished\n\n\
             CRITICAL: A task is COMPLETE when:\n\
             1. You have successfully executed all required tools AND received their results\n\
             2. You have the information/result requested by the user\n\
             3. No further actions are needed to satisfy the user's request\n\n\
             After each tool execution, check: Does the observation contain what the user asked for?\n\
             If YES, immediately set is_final=true and provide the final_answer.\n\
             Do NOT repeat the same action if you already have the result.\n\n\
             Always respond with valid JSON only. No extra text.",
            self.config.system_prompt,
            self.tool_registry.tools_description(),
            context_section,
            max_iterations
        );

        conversation_history.push(ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        });

        conversation_history.push(ChatMessage {
            role: "user".to_string(),
            content: format!("Task: {}", task),
        });

        for iteration in 0..max_iterations {
            let remaining_iterations = max_iterations - iteration;
            tracing::debug!(
                "[{}] Iteration {}/{} (remaining: {})",
                self.config.name,
                iteration + 1,
                max_iterations,
                remaining_iterations
            );

            // Think: Ask LLM for next action
            let decision = match self.think(&conversation_history).await {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("[{}] Failed to get decision: {}", self.config.name, e);
                    return AgentResponse::Failure {
                        error: format!("Failed to reason: {}", e),
                        steps,
                        metadata: None,
                        completion_status: Some(CompletionStatus::Failed {
                            error: format!("LLM reasoning failed: {}", e),
                            recoverable: true,
                        }),
                    };
                }
            };

            tracing::debug!("[{}] Thought: {}", self.config.name, decision.thought);

            // Check if task is complete
            if decision.is_final {
                // If return_tool_output is enabled, use the last tool output instead of LLM's final_answer
                let final_answer = if self.config.return_tool_output {
                    if let Some(tool_output) = &last_tool_output {
                        tracing::debug!(
                            "[{}] Returning last tool output directly",
                            self.config.name
                        );
                        tool_output.clone()
                    } else {
                        tracing::warn!(
                            "[{}] return_tool_output enabled but no tool output available",
                            self.config.name
                        );
                        decision
                            .final_answer
                            .unwrap_or_else(|| "Task completed without tool output".to_string())
                    }
                } else {
                    decision
                        .final_answer
                        .unwrap_or_else(|| "Task completed without explicit answer".to_string())
                };

                steps.push(AgentStep {
                    iteration,
                    thought: decision.thought.clone(),
                    action: None,
                    observation: Some(final_answer.clone()),
                });

                let execution_time = start_time.elapsed().as_millis() as u64;

                return AgentResponse::Success {
                    result: final_answer,
                    steps,
                    metadata: Some(OutputMetadata {
                        confidence: 1.0,
                        execution_time_ms: execution_time,
                        agent_name: Some(self.config.name.clone()),
                        tool_calls: tool_calls.clone(),
                        ..Default::default()
                    }),
                    completion_status: Some(CompletionStatus::Complete { confidence: 1.0 }),
                };
            }

            // Act: Execute the tool
            if let Some(action) = decision.action {
                tracing::info!("[{}] Executing tool: {}", self.config.name, action.tool);

                let tool = match self.tool_registry.get(&action.tool) {
                    Some(t) => t,
                    None => {
                        let error_msg = format!("Tool '{}' not found", action.tool);
                        conversation_history.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: format!("Error: {}", error_msg),
                        });

                        steps.push(AgentStep {
                            iteration,
                            thought: decision.thought,
                            action: Some(action.tool.clone()),
                            observation: Some(error_msg),
                        });
                        continue;
                    }
                };

                // Observe: Get tool result and track execution
                let tool_start = Instant::now();
                let input_size = serde_json::to_string(&action.input)
                    .unwrap_or_default()
                    .len();

                let tool_result = match self.tool_executor.execute(tool, action.input.clone()).await
                {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!("[{}] Tool execution error: {}", self.config.name, e);
                        let error_msg = format!("Tool execution failed: {}", e);

                        // Track failed tool call
                        tool_calls.push(ToolCallMetadata {
                            tool_name: action.tool.clone(),
                            input_size,
                            output_size: error_msg.len(),
                            duration_ms: tool_start.elapsed().as_millis() as u64,
                            success: false,
                        });

                        conversation_history.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: error_msg.clone(),
                        });

                        steps.push(AgentStep {
                            iteration,
                            thought: decision.thought,
                            action: Some(action.tool.clone()),
                            observation: Some(error_msg),
                        });
                        continue;
                    }
                };

                // Track successful tool call
                let output_size = tool_result.output.len();
                tool_calls.push(ToolCallMetadata {
                    tool_name: action.tool.clone(),
                    input_size,
                    output_size,
                    duration_ms: tool_start.elapsed().as_millis() as u64,
                    success: tool_result.success,
                });

                let observation = if tool_result.success {
                    // Store the last successful tool output
                    last_tool_output = Some(tool_result.output.clone());
                    tool_result.output.clone()
                } else {
                    format!("Tool failed: {}", tool_result.error.unwrap_or_default())
                };

                tracing::debug!("[{}] Tool observation: {}", self.config.name, observation);

                // Add the agent's action to conversation history
                conversation_history.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: serde_json::to_string(&AgentDecision {
                        thought: decision.thought.clone(),
                        action: Some(action.clone()),
                        is_final: false,
                        final_answer: None,
                    })
                    .unwrap_or_else(|_| format!("Action: {}", action.tool)),
                });

                // Add observation to conversation with prompt to check completion
                let remaining_after_this = max_iterations - iteration - 1;
                let urgency_msg = if remaining_after_this <= 2 {
                    format!("\n\nWARNING: Only {} iterations remaining! You must complete the task soon or provide a final answer with what you have.", remaining_after_this)
                } else {
                    format!(
                        "\n\nYou have {} iterations remaining.",
                        remaining_after_this
                    )
                };

                conversation_history.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!(
                        "Observation: {}{}\n\nDoes this observation contain the answer to the original task? \
                         If yes, set is_final=true and provide final_answer. \
                         If no, what is the next action needed?",
                        observation, urgency_msg
                    ),
                });

                steps.push(AgentStep {
                    iteration,
                    thought: decision.thought,
                    action: Some(action.tool.clone()),
                    observation: Some(observation),
                });
            } else {
                // No action specified - check if this is actually a completion
                if !steps.is_empty() && steps.iter().any(|s| s.observation.is_some()) {
                    tracing::info!(
                        "[{}] Task appears complete (no new action needed)",
                        self.config.name
                    );

                    // If return_tool_output is enabled, use the last tool output
                    let result = if self.config.return_tool_output {
                        if let Some(tool_output) = &last_tool_output {
                            tracing::debug!(
                                "[{}] Returning last tool output (implicit completion)",
                                self.config.name
                            );
                            tool_output.clone()
                        } else {
                            steps
                                .last()
                                .and_then(|s| s.observation.as_ref())
                                .cloned()
                                .unwrap_or_else(|| "Task completed".to_string())
                        }
                    } else if !decision.thought.is_empty() {
                        decision.thought.clone()
                    } else {
                        steps
                            .last()
                            .and_then(|s| s.observation.as_ref())
                            .cloned()
                            .unwrap_or_else(|| "Task completed".to_string())
                    };

                    steps.push(AgentStep {
                        iteration,
                        thought: "Task completed based on previous observations".to_string(),
                        action: None,
                        observation: Some(result.clone()),
                    });

                    let execution_time = start_time.elapsed().as_millis() as u64;

                    return AgentResponse::Success {
                        result,
                        steps,
                        metadata: Some(OutputMetadata {
                            confidence: 0.8,
                            execution_time_ms: execution_time,
                            agent_name: Some(self.config.name.clone()),
                            tool_calls: tool_calls.clone(),
                            ..Default::default()
                        }),
                        completion_status: Some(CompletionStatus::Complete { confidence: 0.8 }),
                    };
                }

                // Truly no action and no prior work - this is an error
                let error_msg = "No action specified and no prior progress".to_string();
                tracing::warn!("[{}] {}", self.config.name, error_msg);

                conversation_history.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: error_msg.clone(),
                });

                steps.push(AgentStep {
                    iteration,
                    thought: decision.thought,
                    action: None,
                    observation: Some(error_msg),
                });
            }
        }

        // Max iterations reached
        let progress = if steps.is_empty() {
            0.0
        } else {
            (steps.iter().filter(|s| s.observation.is_some()).count() as f32
                / max_iterations as f32)
                .min(0.9)
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        AgentResponse::Timeout {
            partial_result: "Max iterations reached without completing task".to_string(),
            steps,
            metadata: Some(OutputMetadata {
                confidence: progress,
                execution_time_ms: execution_time,
                agent_name: Some(self.config.name.clone()),
                tool_calls,
                ..Default::default()
            }),
            completion_status: Some(CompletionStatus::Partial {
                progress,
                next_steps: vec!["Increase max_iterations or simplify task".to_string()],
            }),
        }
    }

    /// Think step - Ask LLM to reason about next action
    async fn think(&self, conversation: &[ChatMessage]) -> anyhow::Result<AgentDecision> {
        let response = self.llm_client.chat(conversation.to_vec()).await?;

        // Try to parse JSON response
        match serde_json::from_str::<AgentDecision>(&response) {
            Ok(decision) => Ok(decision),
            Err(_e) => {
                // LLM might return text with embedded JSON, try to extract it
                tracing::debug!(
                    "[{}] Response not pure JSON, attempting extraction",
                    self.config.name
                );

                // Try to find JSON in the response
                if let Some(start) = response.find('{') {
                    if let Some(end) = response.rfind('}') {
                        let json_str = &response[start..=end];
                        match serde_json::from_str::<AgentDecision>(json_str) {
                            Ok(decision) => {
                                tracing::debug!(
                                    "[{}] Successfully extracted JSON from response",
                                    self.config.name
                                );
                                return Ok(decision);
                            }
                            Err(_) => {}
                        }
                    }
                }

                // If all parsing fails, create a default decision with the response as thought
                tracing::warn!(
                    "[{}] Could not extract valid JSON, using response as thought",
                    self.config.name
                );
                Ok(AgentDecision {
                    thought: response,
                    action: None,
                    is_final: false,
                    final_answer: None,
                })
            }
        }
    }
}
