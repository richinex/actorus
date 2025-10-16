//! Supervisor Agent - Multi-Agent Orchestration
//!
//! This implements the supervisor pattern from BOOKIDEAS.md Section 12.3:
//! - "Agent of agents" - orchestrates multiple specialized agents
//! - Uses LLM to decompose complex multi-step tasks
//! - Can invoke agents multiple times ("return ticket" pattern)
//! - Coordinates agents to handle complex requests that span domains
//!
//! Information Hiding:
//! - Hides task decomposition logic
//! - Hides agent coordination strategy
//! - Exposes simple orchestration interface

use crate::actors::handoff::HandoffCoordinator;
use crate::actors::messages::{AgentResponse, AgentStep, CompletionStatus};
use crate::actors::specialized_agent::SpecializedAgent;
use crate::config::Settings;
use crate::core::llm::{ChatMessage, LLMClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sub-goal declaration for task planning
#[derive(Debug, Clone, Deserialize, Serialize)]
struct SubGoalDeclaration {
    id: String,
    description: String,
}

/// Supervisor decision returned by LLM
#[derive(Debug, Deserialize, Serialize)]
struct SupervisorDecision {
    thought: String,
    sub_goals: Option<Vec<SubGoalDeclaration>>, // Declare sub-goals upfront (first step only)
    agent_to_invoke: Option<String>,
    agent_task: Option<String>,
    sub_goal_id: Option<String>, // Which sub-goal this task addresses
    is_final: bool,
    final_answer: Option<String>,
}

/// Sub-goal status in the task decomposition
#[derive(Debug, Clone, Serialize, Deserialize)]
enum SubGoalStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// A sub-goal identified by the supervisor
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubGoal {
    id: String,
    description: String,
    status: SubGoalStatus,
    assigned_agent: Option<String>,
    result: Option<String>,
}

/// Task progress tracker for the supervisor
#[derive(Debug, Clone)]
struct TaskProgress {
    sub_goals: Vec<SubGoal>,
    completed_count: usize,
    failed_count: usize,
}

impl TaskProgress {
    fn new() -> Self {
        Self {
            sub_goals: Vec::new(),
            completed_count: 0,
            failed_count: 0,
        }
    }

    fn add_sub_goal(&mut self, id: String, description: String) {
        self.sub_goals.push(SubGoal {
            id,
            description,
            status: SubGoalStatus::Pending,
            assigned_agent: None,
            result: None,
        });
    }

    fn mark_in_progress(&mut self, id: &str, agent: &str) {
        if let Some(goal) = self.sub_goals.iter_mut().find(|g| g.id == id) {
            goal.status = SubGoalStatus::InProgress;
            goal.assigned_agent = Some(agent.to_string());
        }
    }

    fn mark_completed(&mut self, id: &str, result: String) {
        if let Some(goal) = self.sub_goals.iter_mut().find(|g| g.id == id) {
            goal.status = SubGoalStatus::Completed;
            goal.result = Some(result);
            self.completed_count += 1;
        }
    }

    fn mark_failed(&mut self, id: &str, error: String) {
        if let Some(goal) = self.sub_goals.iter_mut().find(|g| g.id == id) {
            goal.status = SubGoalStatus::Failed;
            goal.result = Some(error);
            self.failed_count += 1;
        }
    }

    fn progress_percentage(&self) -> f32 {
        if self.sub_goals.is_empty() {
            0.0
        } else {
            self.completed_count as f32 / self.sub_goals.len() as f32
        }
    }

    fn is_complete(&self) -> bool {
        !self.sub_goals.is_empty() && self.completed_count == self.sub_goals.len()
    }

    fn progress_summary(&self) -> String {
        format!(
            "Progress: {}/{} sub-goals completed ({:.0}%), {} failed",
            self.completed_count,
            self.sub_goals.len(),
            self.progress_percentage() * 100.0,
            self.failed_count
        )
    }

    fn detailed_status(&self) -> String {
        let mut status = String::new();
        status.push_str(&format!(
            "\nTask Progress ({}/{}):\n",
            self.completed_count,
            self.sub_goals.len()
        ));
        for goal in &self.sub_goals {
            let status_icon = match goal.status {
                SubGoalStatus::Pending => "[ ]",
                SubGoalStatus::InProgress => "[→]",
                SubGoalStatus::Completed => "[✓]",
                SubGoalStatus::Failed => "[✗]",
            };
            status.push_str(&format!("  {} {}\n", status_icon, goal.description));
        }
        status
    }
}

/// Supervisor agent that orchestrates multiple specialized agents
pub struct SupervisorAgent {
    agents: HashMap<String, SpecializedAgent>,
    llm_client: LLMClient,
    settings: Settings,
    handoff_coordinator: Option<HandoffCoordinator>,
}

impl SupervisorAgent {
    pub fn new(agents: Vec<SpecializedAgent>, llm_client: LLMClient, settings: Settings) -> Self {
        let mut agent_map = HashMap::new();
        for agent in agents {
            agent_map.insert(agent.name().to_string(), agent);
        }

        Self {
            agents: agent_map,
            llm_client,
            settings,
            handoff_coordinator: None,
        }
    }

    /// Enable handoff validation with a configured coordinator
    pub fn with_handoff_validation(mut self, coordinator: HandoffCoordinator) -> Self {
        self.handoff_coordinator = Some(coordinator);
        self
    }

    /// Orchestrate a complex task across multiple specialized agents
    pub async fn orchestrate(&self, task: &str, max_orchestration_steps: usize) -> AgentResponse {
        tracing::info!("[SupervisorAgent] Orchestrating task: {}", task);

        let mut conversation_history = Vec::new();
        let mut all_steps = Vec::new();
        let mut agent_results: Vec<(String, String)> = Vec::new(); // (agent_name, result)
        let mut agent_results_context: serde_json::Map<String, serde_json::Value> =
            serde_json::Map::new(); // Structured context
        let mut task_progress = TaskProgress::new();

        // Build agent descriptions for the supervisor prompt
        let agent_descriptions: Vec<String> = self
            .agents
            .values()
            .map(|agent| format!("- {}: {}", agent.name(), agent.description()))
            .collect();

        let max_sub_goals = self.settings.agent.max_sub_goals;

        let supervisor_system_prompt = format!(
            "You are a supervisor that coordinates multiple specialized agents to accomplish complex tasks.\n\n\
             Available Agents:\n{}\n\n\
             IMPORTANT LIMITS:\n\
             - Maximum orchestration steps: {}\n\
             - Maximum sub-goals to declare: {}\n\n\
             Your role is to:\n\
             1. IN YOUR FIRST RESPONSE: Analyze the task and declare sub-goals upfront (max {})\n\
             2. IN SUBSEQUENT RESPONSES: Invoke appropriate agents to accomplish each sub-goal\n\
             3. Track progress and combine results to provide a final answer\n\n\
             CRITICAL - Passing Data Between Agents:\n\
             - When an agent produces data that the next agent needs, you MUST include the complete data in the agent_task field\n\
             - For example, if agent A returns JSON data and agent B needs to analyze it, set agent_task to: \"Analyze this data: {{the actual JSON here}}\"\n\
             - Do NOT just reference the data (\"use the data from step 1\") - include the actual data!\n\
             - The agent_task is the ONLY information the agent receives - make it complete\n\n\
             You MUST respond in this EXACT JSON format:\n\
             {{\n  \
               \"thought\": \"your reasoning about what to do next\",\n  \
               \"sub_goals\": [{{\"id\": \"goal_1\", \"description\": \"...\"}}, ...] or null,\n  \
               \"agent_to_invoke\": \"agent_name or null\",\n  \
               \"agent_task\": \"specific task for the agent or null\",\n  \
               \"sub_goal_id\": \"which sub-goal this addresses or null\",\n  \
               \"is_final\": false,\n  \
               \"final_answer\": null\n\
             }}\n\n\
             FIRST STEP (Planning):\n\
             - Declare AT MOST {} sub-goals (prioritize the most important)\n\
             - Set \"sub_goals\" to an array with ids like 'goal_1', 'goal_2', etc.\n\
             - Set \"agent_to_invoke\" to the first agent you'll use\n\
             - Set \"agent_task\" to the specific task for that agent\n\
             - Set \"sub_goal_id\" to 'goal_1' (the first sub-goal)\n\
             - Set \"is_final\" to false\n\n\
             SUBSEQUENT STEPS (Execution):\n\
             - Set \"sub_goals\" to null (only declare once)\n\
             - Set \"agent_to_invoke\" to the next agent\n\
             - Set \"agent_task\" to the specific task\n\
             - Set \"sub_goal_id\" to which goal this addresses (e.g., 'goal_2', 'goal_3')\n\
             - Set \"is_final\" to false\n\n\
             FINAL STEP (Completion):\n\
             - Set \"is_final\" to true when ALL sub-goals are complete\n\
             - Set all other fields to null\n\
             - Provide a comprehensive \"final_answer\" that combines all results\n\n\
             Progress Tracking:\n\
             - You will receive progress updates showing completed sub-goals with checkmarks\n\
             - Use this to decide which sub-goal to work on next\n\
             - When all sub-goals show [✓], provide the final answer\n\n\
             CRITICAL: If the task is complex, prioritize the {} most important sub-goals.\n\
             You can invoke the same agent multiple times if needed.\n\
             Always consider previous agent results when deciding next steps.\n\n\
             Respond with valid JSON only. No extra text.",
            agent_descriptions.join("\n"),
            max_orchestration_steps,
            max_sub_goals,
            max_sub_goals,
            max_sub_goals,
            max_sub_goals
        );

        conversation_history.push(ChatMessage {
            role: "system".to_string(),
            content: supervisor_system_prompt,
        });

        conversation_history.push(ChatMessage {
            role: "user".to_string(),
            content: format!("Task: {}", task),
        });

        for step in 0..max_orchestration_steps {
            let remaining_steps = max_orchestration_steps - step;
            tracing::debug!(
                "[SupervisorAgent] Orchestration step {}/{} (remaining: {})",
                step + 1,
                max_orchestration_steps,
                remaining_steps
            );

            // Ask supervisor what to do next
            let decision = match self.decide_next_action(&conversation_history).await {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("[SupervisorAgent] Failed to get decision: {}", e);
                    return AgentResponse::Failure {
                        error: format!("Supervisor decision failed: {}", e),
                        steps: all_steps,
                        metadata: None,
                        completion_status: Some(CompletionStatus::Failed {
                            error: format!("Supervisor reasoning failed: {}", e),
                            recoverable: true,
                        }),
                    };
                }
            };

            tracing::debug!("[SupervisorAgent] Thought: {}", decision.thought);

            // Handle sub-goal declaration (first step only)
            if let Some(sub_goal_declarations) = decision.sub_goals {
                let declared_count = sub_goal_declarations.len();
                let max_allowed = self.settings.agent.max_sub_goals;

                if declared_count > max_allowed {
                    tracing::warn!(
                        "[SupervisorAgent] LLM declared {} sub-goals, but max_sub_goals is {}. Truncating to first {}.",
                        declared_count,
                        max_allowed,
                        max_allowed
                    );
                }

                let goals_to_add = sub_goal_declarations.into_iter().take(max_allowed);
                let added_count = goals_to_add.len();

                for declaration in goals_to_add {
                    task_progress.add_sub_goal(declaration.id, declaration.description);
                }

                tracing::info!(
                    "[SupervisorAgent] Declared {} sub-goals (max allowed: {})",
                    added_count,
                    max_allowed
                );
                tracing::info!("[SupervisorAgent] {}", task_progress.progress_summary());
                tracing::debug!("[SupervisorAgent] {}", task_progress.detailed_status());
            }

            // Check if all sub-goals are complete (auto-completion)
            if !decision.is_final
                && task_progress.is_complete()
                && !task_progress.sub_goals.is_empty()
            {
                tracing::info!("[SupervisorAgent] All sub-goals completed - auto-completing task");

                // Gather results from all completed sub-goals
                let combined_results: Vec<String> = task_progress
                    .sub_goals
                    .iter()
                    .filter_map(|g| g.result.clone())
                    .collect();

                let final_answer = format!(
                    "Task completed successfully. All sub-goals accomplished:\n{}",
                    combined_results.join("\n")
                );

                all_steps.push(AgentStep {
                    iteration: step,
                    thought: format!(
                        "All sub-goals complete: {}",
                        task_progress.progress_summary()
                    ),
                    action: None,
                    observation: Some(final_answer.clone()),
                });

                return AgentResponse::Success {
                    result: final_answer,
                    steps: all_steps,
                    metadata: None,
                    completion_status: Some(CompletionStatus::Complete { confidence: 0.95 }),
                };
            }

            // Check if task is complete
            if decision.is_final {
                let final_answer = decision
                    .final_answer
                    .unwrap_or_else(|| "Task completed without explicit answer".to_string());

                all_steps.push(AgentStep {
                    iteration: step,
                    thought: decision.thought.clone(),
                    action: None,
                    observation: Some(final_answer.clone()),
                });

                tracing::info!("[SupervisorAgent] Task orchestration complete");

                return AgentResponse::Success {
                    result: final_answer,
                    steps: all_steps,
                    metadata: None,
                    completion_status: Some(CompletionStatus::Complete { confidence: 1.0 }),
                };
            }

            // Invoke agent if specified
            if let (Some(agent_name), Some(agent_task)) = (
                decision.agent_to_invoke.clone(),
                decision.agent_task.clone(),
            ) {
                tracing::info!(
                    "[SupervisorAgent] Invoking '{}' with task: {}",
                    agent_name,
                    agent_task
                );

                // Get sub-goal id
                let sub_goal_id = decision.sub_goal_id.clone().unwrap_or_else(|| {
                    // Fallback: create ad-hoc sub-goal if not specified
                    let fallback_id = format!("goal_{}", step);
                    tracing::warn!(
                        "[SupervisorAgent] No sub_goal_id specified, using fallback: {}",
                        fallback_id
                    );
                    fallback_id
                });

                // Add sub-goal if it doesn't exist (for cases where LLM didn't declare upfront)
                if !task_progress.sub_goals.iter().any(|g| g.id == sub_goal_id) {
                    tracing::warn!(
                        "[SupervisorAgent] Sub-goal '{}' not declared upfront, adding now",
                        sub_goal_id
                    );
                    task_progress.add_sub_goal(sub_goal_id.clone(), agent_task.clone());
                }

                // Mark as in progress
                task_progress.mark_in_progress(&sub_goal_id, &agent_name);

                tracing::info!(
                    "[SupervisorAgent] Working on sub-goal '{}': {}",
                    sub_goal_id,
                    task_progress.progress_summary()
                );

                match self.agents.get(&agent_name) {
                    Some(agent) => {
                        // Build context from previous agent results
                        let context = if !agent_results_context.is_empty() {
                            Some(serde_json::Value::Object(agent_results_context.clone()))
                        } else {
                            None
                        };

                        tracing::debug!(
                            "[SupervisorAgent] Passing context with {} entries to agent '{}'",
                            agent_results_context.len(),
                            agent_name
                        );

                        // Execute agent task with context
                        let agent_response = agent
                            .execute_task_with_context(
                                &agent_task,
                                context,
                                self.settings.agent.max_iterations,
                            )
                            .await;

                        // Validate handoff if coordinator is configured
                        if let Some(coordinator) = &self.handoff_coordinator {
                            // Try to find a contract for this agent
                            let contract_name = format!("{}_handoff", agent_name);

                            // Debug: log what the agent actually returned
                            if let AgentResponse::Success { result, .. } = &agent_response {
                                tracing::debug!(
                                    "[SupervisorAgent] Agent '{}' returned: {}",
                                    agent_name,
                                    result
                                );
                            }

                            let validation =
                                coordinator.validate_handoff(&contract_name, &agent_response);

                            if !validation.valid {
                                tracing::error!(
                                    "[SupervisorAgent] ❌ Handoff validation FAILED for agent '{}'",
                                    agent_name
                                );
                                for error in &validation.errors {
                                    tracing::error!(
                                        "[SupervisorAgent]    ✗ Field '{}': {}",
                                        error.field,
                                        error.message
                                    );
                                }

                                // Mark sub-goal as failed due to validation
                                task_progress.mark_failed(
                                    &sub_goal_id,
                                    format!(
                                        "Validation failed: {}",
                                        validation
                                            .errors
                                            .iter()
                                            .map(|e| format!("{}: {}", e.field, e.message))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    ),
                                );

                                // Add failure step
                                all_steps.push(AgentStep {
                                    iteration: step,
                                    thought: format!(
                                        "Agent '{}' output validation failed",
                                        agent_name
                                    ),
                                    action: Some(format!("{}:{}", agent_name, agent_task)),
                                    observation: Some(format!(
                                        "VALIDATION FAILED: {}",
                                        validation
                                            .errors
                                            .iter()
                                            .map(|e| format!("{}: {}", e.field, e.message))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    )),
                                });

                                // Continue to next step (supervisor can retry or adjust)
                                conversation_history.push(ChatMessage {
                                    role: "user".to_string(),
                                    content: format!(
                                        "Agent '{}' completed but validation FAILED:\n{}\n\n\
                                         The output does not meet quality standards. You should either:\n\
                                         1. Retry with more specific instructions\n\
                                         2. Try a different approach\n\
                                         3. Mark this sub-goal as failed if unrecoverable",
                                        agent_name,
                                        validation.errors.iter()
                                            .map(|e| format!("  ✗ {}: {}", e.field, e.message))
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    ),
                                });

                                continue;
                            } else {
                                tracing::info!(
                                    "[SupervisorAgent] Handoff validation PASSED for agent '{}'",
                                    agent_name
                                );

                                if !validation.warnings.is_empty() {
                                    for warning in &validation.warnings {
                                        tracing::warn!("[SupervisorAgent]    ⚠️  {}", warning);
                                    }
                                }
                            }
                        }

                        let result_summary = match &agent_response {
                            AgentResponse::Success {
                                result,
                                completion_status,
                                ..
                            } => {
                                agent_results.push((agent_name.clone(), result.clone()));
                                task_progress.mark_completed(&sub_goal_id, result.clone());

                                // Store result in context for future agents
                                // Try to parse as JSON, otherwise store as string
                                let result_value =
                                    serde_json::from_str::<serde_json::Value>(result)
                                        .unwrap_or_else(|_| {
                                            serde_json::Value::String(result.clone())
                                        });
                                agent_results_context
                                    .insert(format!("{}_output", agent_name), result_value);
                                tracing::debug!(
                                    "[SupervisorAgent] Stored result from '{}' in context",
                                    agent_name
                                );

                                // Check if all sub-goals are now complete
                                if task_progress.is_complete()
                                    && !task_progress.sub_goals.is_empty()
                                {
                                    tracing::info!("[SupervisorAgent] All sub-goals completed after this success - finalizing");

                                    let combined_results: Vec<String> = task_progress
                                        .sub_goals
                                        .iter()
                                        .filter_map(|g| g.result.clone())
                                        .collect();

                                    let final_answer = format!(
                                        "Task completed successfully. All {} sub-goals accomplished:\n\n{}",
                                        task_progress.sub_goals.len(),
                                        combined_results.join("\n\n")
                                    );

                                    all_steps.push(AgentStep {
                                        iteration: step,
                                        thought: format!(
                                            "Completed sub-goal '{}': {}",
                                            sub_goal_id,
                                            task_progress.progress_summary()
                                        ),
                                        action: Some(format!("{}:{}", agent_name, agent_task)),
                                        observation: Some(result.clone()),
                                    });

                                    return AgentResponse::Success {
                                        result: final_answer,
                                        steps: all_steps,
                                        metadata: None,
                                        completion_status: Some(CompletionStatus::Complete {
                                            confidence: 0.98,
                                        }),
                                    };
                                }

                                let confidence_info =
                                    if let Some(CompletionStatus::Complete { confidence }) =
                                        completion_status
                                    {
                                        format!(" (confidence: {:.2})", confidence)
                                    } else {
                                        String::new()
                                    };
                                format!("SUCCESS{}: {}", confidence_info, result)
                            }
                            AgentResponse::Failure {
                                error,
                                completion_status,
                                ..
                            } => {
                                task_progress.mark_failed(&sub_goal_id, error.clone());
                                let recoverable_info =
                                    if let Some(CompletionStatus::Failed { recoverable, .. }) =
                                        completion_status
                                    {
                                        if *recoverable {
                                            " (recoverable)"
                                        } else {
                                            " (not recoverable)"
                                        }
                                    } else {
                                        ""
                                    };
                                format!("FAILED{}: {}", recoverable_info, error)
                            }
                            AgentResponse::Timeout {
                                partial_result,
                                completion_status,
                                ..
                            } => {
                                task_progress.mark_failed(&sub_goal_id, partial_result.clone());
                                let progress_info =
                                    if let Some(CompletionStatus::Partial { progress, .. }) =
                                        completion_status
                                    {
                                        format!(" (progress: {:.0}%)", progress * 100.0)
                                    } else {
                                        String::new()
                                    };
                                format!("TIMEOUT{}: {}", progress_info, partial_result)
                            }
                        };

                        tracing::info!(
                            "[SupervisorAgent] Agent '{}' result: {}",
                            agent_name,
                            result_summary
                        );

                        // Add supervisor's decision to conversation
                        conversation_history.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: serde_json::to_string(&SupervisorDecision {
                                thought: decision.thought.clone(),
                                sub_goals: None, // Already declared, don't repeat
                                agent_to_invoke: Some(agent_name.clone()),
                                agent_task: Some(agent_task.clone()),
                                sub_goal_id: Some(sub_goal_id.clone()),
                                is_final: false,
                                final_answer: None,
                            })
                            .unwrap_or_else(|_| format!("Invoking {}", agent_name)),
                        });

                        // Add agent result to conversation with progress tracking
                        let remaining_after_this = max_orchestration_steps - step - 1;
                        let urgency_msg = if remaining_after_this <= 2 {
                            format!("\n\nWARNING: Only {} orchestration steps remaining! You must finalize the task soon or provide a final answer with the results you have.", remaining_after_this)
                        } else {
                            format!(
                                "\n\nYou have {} orchestration steps remaining.",
                                remaining_after_this
                            )
                        };

                        let progress_status = task_progress.detailed_status();

                        conversation_history.push(ChatMessage {
                            role: "user".to_string(),
                            content: format!(
                                "Agent '{}' completed the task.\nResult: {}{}\n{}\n\n\
                                 Based on this result and progress, what should happen next?\n\
                                 IMPORTANT: If the next agent needs this result as input, you MUST copy the complete result data into the agent_task field!\n\
                                 If all sub-goals are complete, set is_final=true and provide the final_answer.",
                                agent_name, result_summary, urgency_msg, progress_status
                            ),
                        });

                        all_steps.push(AgentStep {
                            iteration: step,
                            thought: decision.thought,
                            action: Some(format!("{}:{}", agent_name, agent_task)),
                            observation: Some(result_summary),
                        });
                    }
                    None => {
                        let error_msg = format!("Agent '{}' not found", agent_name);
                        tracing::error!("[SupervisorAgent] {}", error_msg);

                        conversation_history.push(ChatMessage {
                            role: "user".to_string(),
                            content: format!("Error: {}", error_msg),
                        });

                        all_steps.push(AgentStep {
                            iteration: step,
                            thought: decision.thought,
                            action: Some(agent_name),
                            observation: Some(error_msg),
                        });
                    }
                }
            } else {
                // No agent specified - supervisor needs to make progress
                let warning =
                    "Supervisor must either invoke an agent or mark task as final".to_string();
                tracing::warn!("[SupervisorAgent] {}", warning);

                conversation_history.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!(
                        "{}\nPlease either:\n\
                         1. Invoke an agent with a specific task, OR\n\
                         2. Set is_final=true if the task is complete",
                        warning
                    ),
                });

                all_steps.push(AgentStep {
                    iteration: step,
                    thought: decision.thought,
                    action: None,
                    observation: Some(warning),
                });
            }
        }

        // Max orchestration steps reached
        tracing::warn!("[SupervisorAgent] Max orchestration steps reached");
        tracing::info!(
            "[SupervisorAgent] Final {}",
            task_progress.progress_summary()
        );

        let progress = task_progress.progress_percentage();

        AgentResponse::Timeout {
            partial_result: format!(
                "Supervisor reached max orchestration steps. {}\nCompleted {} agent invocations.",
                task_progress.progress_summary(),
                agent_results.len()
            ),
            steps: all_steps,
            metadata: None,
            completion_status: Some(CompletionStatus::Partial {
                progress,
                next_steps: vec![
                    "Increase max_orchestration_steps".to_string(),
                    format!("Resume from: {}", task_progress.detailed_status()),
                ],
            }),
        }
    }

    /// Ask supervisor LLM to decide next action
    async fn decide_next_action(
        &self,
        conversation: &[ChatMessage],
    ) -> anyhow::Result<SupervisorDecision> {
        let response = self.llm_client.chat(conversation.to_vec()).await?;

        // Try to parse JSON response
        match serde_json::from_str::<SupervisorDecision>(&response) {
            Ok(decision) => Ok(decision),
            Err(_e) => {
                // LLM might return text with embedded JSON, try to extract it
                tracing::debug!("[SupervisorAgent] Response not pure JSON, attempting extraction");

                // Try to find JSON in the response
                if let Some(start) = response.find('{') {
                    if let Some(end) = response.rfind('}') {
                        let json_str = &response[start..=end];
                        match serde_json::from_str::<SupervisorDecision>(json_str) {
                            Ok(decision) => {
                                tracing::debug!(
                                    "[SupervisorAgent] Successfully extracted JSON from response"
                                );
                                return Ok(decision);
                            }
                            Err(_) => {}
                        }
                    }
                }

                // If all parsing fails, create a default decision
                tracing::warn!(
                    "[SupervisorAgent] Could not extract valid JSON, using response as thought"
                );
                Ok(SupervisorDecision {
                    thought: response,
                    sub_goals: None,
                    agent_to_invoke: None,
                    agent_task: None,
                    sub_goal_id: None,
                    is_final: false,
                    final_answer: None,
                })
            }
        }
    }
}
