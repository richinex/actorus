# Actorus Architecture

This document provides a technical overview of the Actorus architecture, design decisions, and implementation details.

## Table of Contents

1. [Overview](#overview)
2. [Actor Model](#actor-model)
3. [Core Components](#core-components)
4. [Message Flow](#message-flow)
5. [Agent System](#agent-system)
6. [Tool System](#tool-system)
7. [MCP Integration](#mcp-integration)
8. [Validation Framework](#validation-framework)
9. [Design Decisions](#design-decisions)
10. [Performance Considerations](#performance-considerations)

## Overview

Actorus is built on the actor pattern, providing a fault-tolerant, concurrent system for LLM interactions. The architecture separates concerns through isolated actors communicating via message passing.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Public API Layer                         │
│  (Simple async functions: generate_text, supervisor, etc.)  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Message Router                            │
│  (Central hub - routes messages to appropriate actors)      │
└─────────────────────────────────────────────────────────────┘
           │              │              │              │
           ▼              ▼              ▼              ▼
    ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
    │   LLM    │   │  Agent   │   │   MCP    │   │  Health  │
    │  Actor   │   │  Actor   │   │  Actor   │   │ Monitor  │
    └──────────┘   └──────────┘   └──────────┘   └──────────┘
```

## Actor Model

### Core Principles

1. **Isolated State**: Each actor maintains its own state, no shared memory
2. **Message Passing**: All communication happens through messages
3. **Asynchronous**: Non-blocking message handling
4. **Supervision**: Actors can supervise and restart other actors

### Actor Lifecycle

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Create  │ --> │  Active  │ --> │  Failed  │ --> │ Restart  │
└──────────┘     └──────────┘     └──────────┘     └──────────┘
                       │                                  │
                       └──────────────────────────────────┘
```

1. **Create**: Actor is spawned with initial state
2. **Active**: Actor processes messages from its mailbox
3. **Failed**: Actor encounters error
4. **Restart**: Supervisor recreates actor with fresh state

### Message Passing

Messages are sent through channels:

```rust
// Message types
pub enum RoutingMessage {
    GenerateText {
        prompt: String,
        response: oneshot::Sender<Result<String>>,
    },
    RunAgent {
        agent_id: String,
        task: String,
        response: oneshot::Sender<Result<AgentResult>>,
    },
    GetState(oneshot::Sender<StateSnapshot>),
    Shutdown,
}
```

Key characteristics:
- **Type Safety**: Strongly typed messages
- **Response Channels**: One-shot channels for replies
- **Non-Blocking**: Sender doesn't wait for receiver

## Core Components

### 1. Message Router (`actors/message_router.rs`)

Central coordinator for all actor communication.

**Responsibilities**:
- Route messages to appropriate actors
- Maintain actor registry
- Handle actor lifecycle
- Coordinate shutdowns

**Key Code**:
```rust
pub struct MessageRouterHandle {
    sender: mpsc::Sender<RoutingMessage>,
}

impl MessageRouterHandle {
    pub async fn send_message(&self, msg: RoutingMessage) -> Result<()> {
        self.sender.send(msg).await?;
        Ok(())
    }
}
```

### 2. LLM Actor (`actors/llm_actor.rs`)

Handles OpenAI API interactions.

**Responsibilities**:
- Make API requests
- Handle streaming
- Manage rate limits
- Parse responses

**Key Features**:
- Async HTTP client
- Retry logic
- Error handling
- Response streaming

### 3. Agent Actor (`actors/agent_actor.rs`)

Executes specialized agent tasks.

**Responsibilities**:
- Execute agent logic
- Manage tool calls
- Handle agent state
- Report results

**Key Features**:
- Tool execution
- ReAct loop implementation
- Function calling
- Result aggregation

### 4. Supervisor Agent (`actors/supervisor_agent.rs`)

Orchestrates multi-agent workflows.

**Responsibilities**:
- Break down complex tasks
- Select appropriate agents
- Aggregate results
- Handle failures

**Key Features**:
- LLM-powered orchestration
- Sub-goal tracking
- Agent selection
- Result synthesis

### 5. MCP Actor (`actors/mcp_actor.rs`)

Interfaces with Model Context Protocol servers.

**Responsibilities**:
- Spawn MCP server processes
- Handle JSON-RPC communication
- Manage tool discovery
- Execute tool calls

**Key Features**:
- Process management
- stdin/stdout communication
- Tool wrapping
- Error handling

### 6. Health Monitor (`actors/health_monitor.rs`)

Tracks actor health and enables recovery.

**Responsibilities**:
- Monitor actor heartbeats
- Detect failures
- Trigger restarts
- Report system state

**Key Features**:
- Periodic health checks
- Timeout detection
- Restart coordination
- State snapshots

## Message Flow

### Basic Text Generation

```
User Code
    │
    │ generate_text()
    ▼
Message Router
    │
    │ GenerateText message
    ▼
LLM Actor
    │
    │ OpenAI API call
    ▼
Response
    │
    │ Return via oneshot
    ▼
User Code
```

### Multi-Agent Orchestration

```
User Code
    │
    │ supervisor::orchestrate()
    ▼
Message Router
    │
    │ RunSupervisor message
    ▼
Supervisor Agent
    │
    ├──> Sub-goal 1 ──> Agent A ──> LLM Actor ──> Result 1
    │
    ├──> Sub-goal 2 ──> Agent B ──> MCP Actor ──> Result 2
    │
    └──> Sub-goal 3 ──> Agent C ──> LLM Actor ──> Result 3
    │
    │ Aggregate results
    ▼
User Code
```

### MCP Tool Call

```
Agent
    │
    │ Tool call decision
    ▼
MCP Actor
    │
    ├──> Spawn npx process
    │
    ├──> Send JSON-RPC request
    │
    ├──> Read response
    │
    └──> Parse result
    │
    ▼
Agent
```

## Agent System

### Agent Builder Pattern

Fluent API for agent construction:

```rust
let agent = AgentBuilder::new("agent_name")
    .description("Agent description")
    .system_prompt("System prompt")
    .tool(Tool1::new())
    .tool(Tool2::new())
    .build();
```

**Key Features**:
- Fluent interface
- Type-safe construction
- Tool composition
- Configuration validation

### Agent Configuration

```rust
pub struct AgentConfig {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub tools: Vec<Arc<dyn Tool>>,
    pub response_format: Option<ResponseFormat>,
    pub allow_delegation: bool,
}
```

### ReAct Loop

Agents use the ReAct (Reasoning + Acting) pattern:

```
1. Thought: Agent reasons about what to do
2. Action: Agent selects and executes a tool
3. Observation: Agent observes tool result
4. Repeat until task complete
```

Implementation:
```rust
loop {
    // Get LLM decision
    let response = llm_call_with_tools(context).await?;

    if let Some(tool_call) = response.tool_calls {
        // Execute tool
        let result = execute_tool(tool_call).await?;

        // Add to context
        context.add_tool_result(result);
    } else {
        // Task complete
        return response.content;
    }
}
```

## Tool System

### Tool Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn metadata(&self) -> ToolMetadata;
    async fn execute(&self, args: Value) -> Result<ToolResult>;
}
```

### Tool Metadata

```rust
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
}
```

### Tool Macros

Two styles for tool creation:

**1. Struct-based**:
```rust
#[tool(name = "calculator", description = "Calculate")]
struct Calculator {}

#[async_trait]
impl Tool for Calculator {
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        // Implementation
    }
}
```

**2. Function-based**:
```rust
#[tool_fn(name = "search", description = "Search")]
async fn search(_query: String) -> Result<String> {
    // Implementation
}
```

The macro generates:
- Tool struct
- Metadata implementation
- Parameter extraction
- Error handling

## MCP Integration

### MCP Client

```rust
pub struct MCPClient {
    process: Child,       // Spawned server process
    request_id: u64,      // Request counter
}
```

**Lifecycle**:
1. Spawn server process
2. Initialize with protocol version
3. Send JSON-RPC requests via stdin
4. Read JSON-RPC responses from stdout
5. Clean up process on drop

### Tool Discovery

```rust
pub async fn discover_mcp_tools(
    server_command: &str,
    server_args: Vec<&str>,
) -> Result<Vec<Arc<dyn Tool>>>
```

**Process**:
1. Connect to MCP server
2. Call `tools/list` endpoint
3. Parse tool definitions
4. Create `MCPToolWrapper` for each tool
5. Return ready-to-use tools

### MCPToolWrapper

Converts MCP tools to agent tools:

```rust
pub struct MCPToolWrapper {
    tool_name: String,
    description: String,
    input_schema: Value,      // JSON schema
    server_command: String,
    server_args: Vec<String>,
}
```

**Key Feature**: Creates new MCP client for each execution, ensuring isolation.

## Validation Framework

### Handoff Contracts

Define expected data structure between agents:

```rust
pub struct HandoffContract {
    pub from_agent: String,
    pub to_agent: Option<String>,
    pub schema: OutputSchema,
}

pub struct OutputSchema {
    pub required_fields: Vec<String>,
    pub validation_rules: Vec<ValidationRule>,
}
```

### Validation Rules

```rust
pub enum ValidationType {
    Type,         // Type checking
    Range,        // Numeric ranges
    Enum,         // Allowed values
    Pattern,      // Regex patterns
    Custom,       // Custom validators
}
```

### Quality Gates

Validation happens at agent boundaries:

```
Agent A ──> Output ──> Validator ──> Pass/Fail ──> Agent B
```

If validation fails:
- Log the error
- Optionally retry
- Report to supervisor

## Design Decisions

### 1. Actor Model vs. Shared State

**Decision**: Use actor model with message passing

**Rationale**:
- No shared state eliminates data races
- Natural fault isolation
- Easy to reason about concurrency
- Scales horizontally

**Trade-off**: Slightly more complex than shared state, but much safer

### 2. Message Router vs. Direct Actor Communication

**Decision**: Centralized message router

**Rationale**:
- Single point for actor discovery
- Easier supervision
- Simplified lifecycle management
- Better observability

**Trade-off**: Single point of coordination, but simpler overall

### 3. Synchronous API with Async Actors

**Decision**: Expose async API backed by actors

**Rationale**:
- Simple for users
- Reliability without complexity
- Actors handle failures internally
- Best of both worlds

**Trade-off**: None - users get simplicity and reliability

### 4. MCP Tool Isolation

**Decision**: Create new MCP client for each tool call

**Rationale**:
- Complete isolation between calls
- No state leakage
- Cleaner error handling
- Simpler implementation

**Trade-off**: Slightly slower, but safer and simpler

### 5. Dynamic Tool Discovery

**Decision**: Runtime tool discovery from MCP servers

**Rationale**:
- No hardcoded tool definitions
- Plug-and-play architecture
- Extensible without recompilation
- Scales to any MCP server

**Trade-off**: Runtime overhead, but huge flexibility gain

## Performance Considerations

### Concurrency

- **Actor Parallelism**: Multiple actors run concurrently
- **Tokio Runtime**: Efficient async task scheduling
- **Message Batching**: Group messages when possible
- **Tool Parallelism**: Independent tools run concurrently

### Resource Management

- **Bounded Channels**: Prevent unbounded memory growth
- **Backpressure**: Handle slow consumers
- **Process Cleanup**: Ensure MCP processes are killed
- **Connection Pooling**: Reuse HTTP connections

### Optimization Points

1. **Message Passing**: Fast in-memory channels
2. **JSON Parsing**: Efficient serde operations
3. **HTTP Requests**: Async reqwest with connection reuse
4. **Tool Execution**: Concurrent when independent

### Bottlenecks

1. **OpenAI API**: Rate limits and latency
2. **MCP Process Spawn**: Process creation overhead
3. **JSON-RPC Communication**: stdin/stdout serialization
4. **Tool Execution**: External tool performance

## Future Enhancements

### Planned Features

1. **Distributed Actors**: Actors across multiple machines
2. **Persistent Actors**: Actor state persistence
3. **Circuit Breakers**: Prevent cascade failures
4. **Metrics**: Prometheus-style metrics
5. **Tracing**: Distributed tracing support

### Scalability

- **Horizontal**: Add more actor instances
- **Vertical**: Increase resources per actor
- **Geographic**: Deploy actors globally

## References

- [Actor Model](https://en.wikipedia.org/wiki/Actor_model)
- [Erlang OTP](https://www.erlang.org/doc/design_principles/des_princ.html)
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [ReAct Pattern](https://arxiv.org/abs/2210.03629)
- [Information Hiding (Parnas 1972)](https://dl.acm.org/doi/10.1145/361598.361623)

## See Also

- [ACTOR_AGENTS.md](./ACTOR_AGENTS.md) - Detailed actor pattern documentation
- [EXAMPLES.md](./EXAMPLES.md) - Example programs
- [CONTRIBUTING.md](./CONTRIBUTING.md) - Contribution guidelines
- [README.md](./README.md) - Project overview
