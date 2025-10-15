//! Router Agent - LLM-based Intent Classification and Routing
//!
//! This implements the router pattern from BOOKIDEAS.md Section 12.2:
//! - Receives user message
//! - Uses LLM with structured output to classify intent
//! - Routes to appropriate specialized agent
//! - "One-way ticket" pattern - each query routed once
//!
//! Information Hiding:
//! - Hides intent classification logic
//! - Hides agent selection strategy
//! - Exposes simple routing interface

use crate::actors::messages::{AgentResponse, CompletionStatus};
use crate::actors::specialized_agent::SpecializedAgent;
use crate::core::llm::{ChatMessage, LLMClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Routing decision returned by LLM
#[derive(Debug, Deserialize, Serialize)]
struct RoutingDecision {
    agent_name: String,
    reasoning: String,
}

/// Router agent that classifies intent and routes to specialized agents
pub struct RouterAgent {
    agents: HashMap<String, SpecializedAgent>,
    llm_client: LLMClient,
}

impl RouterAgent {
    pub fn new(agents: Vec<SpecializedAgent>, llm_client: LLMClient) -> Self {
        let mut agent_map = HashMap::new();
        for agent in agents {
            agent_map.insert(agent.name().to_string(), agent);
        }

        Self {
            agents: agent_map,
            llm_client,
        }
    }

    /// Route a task to the appropriate specialized agent
    pub async fn route_task(&self, task: &str, max_iterations: usize) -> AgentResponse {
        tracing::info!("[RouterAgent] Routing task: {}", task);

        // Step 1: Classify intent using LLM
        let routing_decision = match self.classify_intent(task).await {
            Ok(decision) => decision,
            Err(e) => {
                tracing::error!("[RouterAgent] Failed to classify intent: {}", e);
                return AgentResponse::Failure {
                    error: format!("Failed to classify intent: {}", e),
                    steps: vec![],
                    metadata: None,
                    completion_status: Some(CompletionStatus::Failed {
                        error: format!("Intent classification failed: {}", e),
                        recoverable: true,
                    }),
                };
            }
        };

        tracing::info!(
            "[RouterAgent] Routing to '{}' - Reason: {}",
            routing_decision.agent_name,
            routing_decision.reasoning
        );

        // Step 2: Route to selected agent
        match self.agents.get(&routing_decision.agent_name) {
            Some(agent) => {
                agent.execute_task(task, max_iterations).await
            }
            None => {
                tracing::error!(
                    "[RouterAgent] Agent '{}' not found",
                    routing_decision.agent_name
                );

                // Fallback: use general_agent if available
                if let Some(general_agent) = self.agents.get("general_agent") {
                    tracing::info!("[RouterAgent] Falling back to general_agent");
                    general_agent.execute_task(task, max_iterations).await
                } else {
                    AgentResponse::Failure {
                        error: format!(
                            "Agent '{}' not found and no fallback available",
                            routing_decision.agent_name
                        ),
                        steps: vec![],
                        metadata: None,
                        completion_status: Some(CompletionStatus::Failed {
                            error: format!("No suitable agent found for routing"),
                            recoverable: false,
                        }),
                    }
                }
            }
        }
    }

    /// Classify user intent using LLM to determine which agent should handle the task
    async fn classify_intent(&self, task: &str) -> anyhow::Result<RoutingDecision> {
        // Build agent descriptions for the router prompt
        let agent_descriptions: Vec<String> = self
            .agents
            .values()
            .map(|agent| format!("- {}: {}", agent.name(), agent.description()))
            .collect();

        let router_system_prompt = format!(
            "You are a router that classifies user requests and determines which specialized agent should handle them.\n\n\
             Available Agents:\n{}\n\n\
             Your task is to analyze the user's request and decide which agent is best suited to handle it.\n\n\
             IMPORTANT: You MUST respond in this EXACT JSON format:\n\
             {{\n  \
               \"agent_name\": \"the_agent_name\",\n  \
               \"reasoning\": \"why this agent is the best choice\"\n\
             }}\n\n\
             Guidelines:\n\
             - If the task involves file operations (reading/writing files), choose 'file_ops_agent'\n\
             - If the task involves shell commands or system operations, choose 'shell_agent'\n\
             - If the task involves web requests or fetching online data, choose 'web_agent'\n\
             - If the task requires multiple tool types or is unclear, choose 'general_agent'\n\n\
             Respond with valid JSON only. No extra text.",
            agent_descriptions.join("\n")
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: router_system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("Task: {}", task),
            },
        ];

        let response = self.llm_client.chat(messages).await?;

        // Try to parse JSON response
        match serde_json::from_str::<RoutingDecision>(&response) {
            Ok(decision) => Ok(decision),
            Err(e) => {
                // LLM might return text instead of JSON, try to extract JSON
                tracing::warn!("[RouterAgent] Failed to parse decision as JSON: {}", e);

                // Try to find JSON in the response
                if let Some(start) = response.find('{') {
                    if let Some(end) = response.rfind('}') {
                        let json_str = &response[start..=end];
                        match serde_json::from_str::<RoutingDecision>(json_str) {
                            Ok(decision) => return Ok(decision),
                            Err(_) => {}
                        }
                    }
                }

                // If all parsing fails, default to general_agent
                Ok(RoutingDecision {
                    agent_name: "general_agent".to_string(),
                    reasoning: "Failed to parse router response, using general agent as fallback"
                        .to_string(),
                })
            }
        }
    }
}
