use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActorType {
    LLM,
    MCP,
    Agent,
    Router,
    Supervisor,
}

#[derive(Debug)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessageData>,
    pub stream: bool,
    pub response: oneshot::Sender<ChatResponse>,
}

#[derive(Debug, Clone)]
pub struct ChatMessageData {
    pub role: String,
    pub content: String,
}

#[derive(Debug)]
pub enum ChatResponse {
    Complete(String),
    StreamTokens(mpsc::Receiver<String>),
    Error(String),
}

#[derive(Debug)]
pub struct MCPToolCall {
    pub server_command: String,
    pub server_args: Vec<String>,
    pub tool_name: String,
    pub arguments: Value,
    pub response: oneshot::Sender<MCPResponse>,
}

#[derive(Debug)]
pub struct MCPListTools {
    pub server_command: String,
    pub server_args: Vec<String>,
    pub response: oneshot::Sender<MCPResponse>,
}

#[derive(Debug)]
pub enum MCPResponse {
    Tools(Vec<String>),
    Content(String),
    Error(String),
}

#[derive(Debug)]
pub enum LLMMessage {
    Chat(ChatRequest),
}

#[derive(Debug)]
pub enum MCPMessage {
    ListTools(MCPListTools),
    CallTool(MCPToolCall),
}

// Agent-related messages
#[derive(Debug)]
pub struct AgentTask {
    pub task_description: String,
    pub max_iterations: Option<usize>,
    pub response: oneshot::Sender<AgentResponse>,
}

#[derive(Debug, Clone)]
pub struct AgentStep {
    pub iteration: usize,
    pub thought: String,
    pub action: Option<String>,
    pub observation: Option<String>,
}

/// Schema definition for structured agent outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct OutputSchema {
    pub schema_version: String,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub field_types: HashMap<String, String>,
    pub validation_rules: Vec<ValidationRule>,
}

/// Validation rule for output fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ValidationRule {
    pub field: String,
    pub rule_type: ValidationType,
    pub constraint: String,
}

/// Types of validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum ValidationType {
    MinLength,
    MaxLength,
    Pattern,
    Range,
    Enum,
    Custom,
}

/// Validation result with detailed feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub error_type: String,
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }
}

/// Metadata about agent execution and output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub confidence: f32,
    pub execution_time_ms: u64,
    pub tokens_used: Option<u32>,
    pub partial_results: HashMap<String, String>,
    pub schema_version: Option<String>,
    pub validation_result: Option<ValidationResult>,
    pub agent_name: Option<String>,
    pub tool_calls: Vec<ToolCallMetadata>,
}

/// Metadata about tool calls made during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallMetadata {
    pub tool_name: String,
    pub input_size: usize,
    pub output_size: usize,
    pub duration_ms: u64,
    pub success: bool,
}

impl Default for OutputMetadata {
    fn default() -> Self {
        Self {
            confidence: 1.0,
            execution_time_ms: 0,
            tokens_used: None,
            partial_results: HashMap::new(),
            schema_version: None,
            validation_result: None,
            agent_name: None,
            tool_calls: Vec::new(),
        }
    }
}

/// Completion status with additional context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionStatus {
    Complete { confidence: f32 },
    Partial { progress: f32, next_steps: Vec<String> },
    Blocked { reason: String, needs: Vec<String> },
    Failed { error: String, recoverable: bool },
}

#[derive(Debug)]
pub enum AgentResponse {
    Success {
        result: String,
        steps: Vec<AgentStep>,
        metadata: Option<OutputMetadata>,
        completion_status: Option<CompletionStatus>,
    },
    Failure {
        error: String,
        steps: Vec<AgentStep>,
        metadata: Option<OutputMetadata>,
        completion_status: Option<CompletionStatus>,
    },
    Timeout {
        partial_result: String,
        steps: Vec<AgentStep>,
        metadata: Option<OutputMetadata>,
        completion_status: Option<CompletionStatus>,
    },
}

#[derive(Debug)]
pub enum AgentMessage {
    RunTask(AgentTask),
    Stop,
}

#[derive(Debug)]
pub enum RoutingMessage {
    LLM(LLMMessage),
    MCP(MCPMessage),
    Agent(AgentMessage),
    Heartbeat(ActorType),
    Reset(ActorType),
    GetState(oneshot::Sender<StateSnapshot>),
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub active_actors: HashMap<ActorType, bool>,
    pub last_heartbeat: HashMap<ActorType, Instant>,
}
