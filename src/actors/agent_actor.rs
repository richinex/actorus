//! Agent Actor - Autonomous ReAct (Reason + Act) Agent
//!
//! Information Hiding:
//! - ReAct loop implementation details hidden
//! - Tool selection logic hidden
//! - State management internalized
//! - LLM interaction details abstracted

use crate::actors::messages::*;
use crate::config::Settings;
use crate::core::llm::{ChatMessage, LLMClient};
use crate::tools::{registry::ToolRegistry, executor::ToolExecutor, ToolConfig};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{interval, Duration};

static ROUTER_SENDER: OnceCell<Sender<RoutingMessage>> = OnceCell::new();

pub fn set_router_sender(sender: Sender<RoutingMessage>) {
    let _ = ROUTER_SENDER.set(sender);
}

/// Handle for communicating with the agent actor
pub struct AgentActorHandle {
    sender: Sender<AgentMessage>,
}

impl AgentActorHandle {
    pub fn new(settings: Settings, api_key: String) -> Self {
        let buffer_size = settings.system.channel_buffer_size;
        let (sender, receiver) = channel(buffer_size);

        tokio::spawn(agent_actor(receiver, settings, api_key));

        Self { sender }
    }

    pub async fn send_message(&self, message: AgentMessage) -> anyhow::Result<()> {
        self.sender
            .send(message)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send message to Agent actor: {}", e))
    }
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

/// Agent actor implementation - ReAct pattern
async fn agent_actor(
    mut receiver: Receiver<AgentMessage>,
    settings: Settings,
    api_key: String,
) {
    tracing::info!("Agent actor started");

    let llm_client = LLMClient::new(api_key, settings.clone());
    let tool_registry = Arc::new(ToolRegistry::with_defaults());
    let tool_executor = ToolExecutor::new(ToolConfig::default());

    let heartbeat_interval = Duration::from_millis(settings.system.heartbeat_interval_ms);
    let mut heartbeat_timer = interval(heartbeat_interval);

    // Get default max_iterations from config
    let default_max_iterations = settings.agent.max_iterations;

    loop {
        tokio::select! {
            Some(message) = receiver.recv() => {
                match message {
                    AgentMessage::RunTask(task) => {
                        tracing::info!("Agent received task: {}", task.task_description);

                        let result = run_react_loop(
                            &llm_client,
                            &tool_registry,
                            &tool_executor,
                            &task.task_description,
                            task.max_iterations.unwrap_or(default_max_iterations),
                        ).await;

                        let _ = task.response.send(result);
                    }
                    AgentMessage::Stop => {
                        tracing::info!("Agent actor stopping");
                        break;
                    }
                }
            }

            _ = heartbeat_timer.tick() => {
                if let Some(sender) = ROUTER_SENDER.get() {
                    let _ = sender.send(RoutingMessage::Heartbeat(ActorType::Agent)).await;
                    tracing::trace!("Agent sent heartbeat");
                }
            }
        }
    }

    tracing::info!("Agent actor stopped");
}

/// Run the ReAct (Reason + Act) loop
///
/// This is the core autonomous agent loop:
/// 1. Think: Use LLM to reason about next action
/// 2. Act: Execute selected tool
/// 3. Observe: Get tool result
/// 4. Repeat until goal achieved or max iterations reached
async fn run_react_loop(
    llm_client: &LLMClient,
    tool_registry: &ToolRegistry,
    tool_executor: &ToolExecutor,
    task: &str,
    max_iterations: usize,
) -> AgentResponse {
    let mut steps = Vec::new();
    let mut conversation_history = Vec::new();

    // System prompt for the agent
    let system_prompt = format!(
        "You are an autonomous agent that can use tools to accomplish tasks.\n\n\
         Available Tools:\n{}\n\n\
         IMPORTANT: You MUST respond in this EXACT JSON format:\n\
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
        tool_registry.tools_description()
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
        tracing::info!("Agent iteration {}/{}", iteration + 1, max_iterations);

        // Think: Ask LLM for next action
        let decision = match think(llm_client, &conversation_history).await {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to get decision from LLM: {}", e);
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

        tracing::debug!("Agent thought: {}", decision.thought);

        // Check if task is complete
        if decision.is_final {
            let final_answer = decision.final_answer.unwrap_or_else(|| {
                "Task completed without explicit answer".to_string()
            });

            steps.push(AgentStep {
                iteration,
                thought: decision.thought.clone(),
                action: None,
                observation: Some(final_answer.clone()),
            });

            return AgentResponse::Success {
                result: final_answer,
                steps,
                metadata: None,
                completion_status: Some(CompletionStatus::Complete { confidence: 1.0 }),
            };
        }

        // Act: Execute the tool
        if let Some(action) = decision.action {
            tracing::info!("Agent executing tool: {}", action.tool);

            let tool = match tool_registry.get(&action.tool) {
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

            // Observe: Get tool result
            let tool_result = match tool_executor.execute(tool, action.input.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Tool execution error: {}", e);
                    let error_msg = format!("Tool execution failed: {}", e);
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

            let observation = if tool_result.success {
                tool_result.output.clone()
            } else {
                format!("Tool failed: {}", tool_result.error.unwrap_or_default())
            };

            tracing::debug!("Tool observation: {}", observation);

            // Add the agent's action to conversation history
            conversation_history.push(ChatMessage {
                role: "assistant".to_string(),
                content: serde_json::to_string(&AgentDecision {
                    thought: decision.thought.clone(),
                    action: Some(action.clone()),
                    is_final: false,
                    final_answer: None,
                }).unwrap_or_else(|_| format!("Action: {}", action.tool)),
            });

            // Add observation to conversation with prompt to check completion
            conversation_history.push(ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Observation: {}\n\nDoes this observation contain the answer to the original task? \
                     If yes, set is_final=true and provide final_answer. \
                     If no, what is the next action needed?",
                    observation
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
            // If we have previous observations and no action, treat as complete
            if !steps.is_empty() && steps.iter().any(|s| s.observation.is_some()) {
                tracing::info!("Agent appears to have completed task (no new action needed)");

                // Extract summary from thought or last observation
                let result = if !decision.thought.is_empty() {
                    decision.thought.clone()
                } else {
                    steps.last()
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

                return AgentResponse::Success {
                    result,
                    steps,
                    metadata: None,
                    completion_status: Some(CompletionStatus::Complete { confidence: 0.8 }),
                };
            }

            // Truly no action and no prior work - this is an error
            let error_msg = "No action specified and no prior progress".to_string();
            tracing::warn!("{}", error_msg);

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
        (steps.iter().filter(|s| s.observation.is_some()).count() as f32 / max_iterations as f32).min(0.9)
    };

    AgentResponse::Timeout {
        partial_result: "Max iterations reached without completing task".to_string(),
        steps,
        metadata: None,
        completion_status: Some(CompletionStatus::Partial {
            progress,
            next_steps: vec!["Increase max_iterations or simplify task".to_string()],
        }),
    }
}

/// Think step - Ask LLM to reason about next action
async fn think(
    llm_client: &LLMClient,
    conversation: &[ChatMessage],
) -> anyhow::Result<AgentDecision> {
    let response = llm_client.chat(conversation.to_vec()).await?;

    // Try to parse JSON response
    match serde_json::from_str::<AgentDecision>(&response) {
        Ok(decision) => Ok(decision),
        Err(e) => {
            // LLM might return text instead of JSON, try to extract JSON
            tracing::warn!("Failed to parse decision as JSON: {}", e);

            // Try to find JSON in the response
            if let Some(start) = response.find('{') {
                if let Some(end) = response.rfind('}') {
                    let json_str = &response[start..=end];
                    match serde_json::from_str::<AgentDecision>(json_str) {
                        Ok(decision) => return Ok(decision),
                        Err(_) => {}
                    }
                }
            }

            // If all parsing fails, create a default decision with the response as thought
            Ok(AgentDecision {
                thought: response,
                action: None,
                is_final: false,
                final_answer: None,
            })
        }
    }
}
