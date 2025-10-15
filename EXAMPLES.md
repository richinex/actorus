# Actorus Examples Guide

This document provides a comprehensive guide to all examples in the project. Examples are organized by category and difficulty level.

## Running Examples

```bash
cargo run --example <example_name>
```

For MCP examples, ensure you have the required MCP servers installed:
```bash
npm install -g @modelcontextprotocol/server-brave-search
npm install -g @modelcontextprotocol/server-filesystem
```

## Basic Examples

### simple_usage.rs
**Difficulty**: Beginner
**Purpose**: Demonstrates basic text generation

Shows the simplest way to use Actorus for text generation.

```rust
use actorus::{init, generate_text};

init().await?;
let response = generate_text("Explain Rust ownership", None).await?;
```

**Key Concepts**:
- System initialization
- Basic text generation
- Error handling

**Run**: `cargo run --example simple_usage`

### advanced_usage.rs
**Difficulty**: Intermediate
**Purpose**: Shows advanced features like streaming, retries, and fallback

Demonstrates more complex API usage patterns.

**Features**:
- Streaming responses
- Retry logic
- Fallback strategies
- Custom configurations

**Run**: `cargo run --example advanced_usage`

### batch_processing.rs
**Difficulty**: Intermediate
**Purpose**: Process multiple prompts concurrently

Shows how to efficiently process multiple prompts with concurrency control.

**Key Concepts**:
- Concurrent request handling
- Backpressure management
- Result aggregation

**Run**: `cargo run --example batch_processing`

## Agent Examples

### agent_usage.rs
**Difficulty**: Intermediate
**Purpose**: Create and use specialized agents

Introduction to the agent system.

```rust
let agent = AgentBuilder::new("researcher")
    .description("Research specialist")
    .system_prompt("You are a research expert.")
    .build();
```

**Key Concepts**:
- Agent creation with AgentBuilder
- Agent configuration
- System prompts

**Run**: `cargo run --example agent_usage`

### supervisor_usage.rs
**Difficulty**: Intermediate
**Purpose**: Multi-agent orchestration

Demonstrates the supervisor pattern for coordinating multiple agents.

**Key Concepts**:
- Supervisor orchestration
- Multi-agent workflows
- Task decomposition

**Run**: `cargo run --example supervisor_usage`

### supervisor_custom_agents.rs
**Difficulty**: Advanced
**Purpose**: Custom agent pipeline with supervisor

Shows how to create custom specialized agents and orchestrate them.

**Key Concepts**:
- Custom agent design
- Agent specialization
- Pipeline architecture

**Run**: `cargo run --example supervisor_custom_agents`

### agent_introspection.rs
**Difficulty**: Intermediate
**Purpose**: Query agent capabilities and state

Learn how to inspect agent capabilities at runtime.

**Key Concepts**:
- Agent introspection
- Capability discovery
- Runtime queries

**Run**: `cargo run --example agent_introspection`

### simplified_agent_builder.rs
**Difficulty**: Beginner
**Purpose**: Simple agent creation patterns

Demonstrates the easiest way to create agents.

**Run**: `cargo run --example simplified_agent_builder`

## Tool Examples

### tool_with_macro.rs
**Difficulty**: Intermediate
**Purpose**: Create tools using macros

Shows how to define custom tools with the `#[tool]` macro.

```rust
#[tool(
    name = "calculator",
    description = "Performs basic calculations"
)]
struct Calculator {}

#[async_trait]
impl Tool for Calculator {
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        // Implementation
    }
}
```

**Key Concepts**:
- Tool macro usage
- Tool metadata
- Tool execution

**Run**: `cargo run --example tool_with_macro`

### tool_function_style.rs
**Difficulty**: Intermediate
**Purpose**: Function-style tool definition

Demonstrates the `#[tool_fn]` macro for simpler tool creation.

```rust
#[tool_fn(
    name = "search",
    description = "Search the web"
)]
async fn search(_query: String) -> Result<String> {
    // Implementation
}
```

**Key Concepts**:
- Function-style tools
- Simpler syntax
- Automatic wrapping

**Run**: `cargo run --example tool_function_style`

### advanced_tool_macro.rs
**Difficulty**: Advanced
**Purpose**: Complex tool patterns

Shows advanced tool creation patterns.

**Run**: `cargo run --example advanced_tool_macro`

### supervisor_with_custom_tools.rs
**Difficulty**: Advanced
**Purpose**: Agents with custom tools in supervisor

Combines custom tools with supervisor orchestration.

**Run**: `cargo run --example supervisor_with_custom_tools`

## MCP Examples

### mcp_simple_demo.rs
**Difficulty**: Beginner
**Purpose**: Basic MCP server interaction

Simplest MCP example showing server connection and tool listing.

```rust
let mut client = MCPClient::new(
    "npx",
    vec!["-y", "@modelcontextprotocol/server-brave-search"]
).await?;

let tools = client.list_tools().await?;
```

**Prerequisites**:
- `npm install -g @modelcontextprotocol/server-brave-search`
- `export BRAVE_API_KEY=your_key`

**Key Concepts**:
- MCP client creation
- Tool listing
- Basic tool calls

**Run**: `cargo run --example mcp_simple_demo`

### mcp_discover_tools.rs
**Difficulty**: Intermediate
**Purpose**: Dynamic tool discovery from MCP servers

Shows how to discover tools without running full agent pipeline.

```rust
let tools = discover_mcp_tools(
    "npx",
    vec!["-y", "@modelcontextprotocol/server-brave-search"]
).await?;
```

**Prerequisites**:
- MCP server installed (Brave Search or Filesystem)

**Key Concepts**:
- Dynamic tool discovery
- Tool metadata extraction
- No agent execution needed

**Run**: `cargo run --example mcp_discover_tools`

### supervisor_dynamic_mcp_tools.rs
**Difficulty**: Advanced
**Purpose**: Full agent integration with dynamic MCP tools

Complete example of discovering MCP tools and using them in agents.

```rust
// Discover tools
let tools = discover_mcp_tools("npx", vec!["-y", "server"]).await?;

// Create agent with tools
let mut agent = AgentBuilder::new("research_agent");
for tool in tools {
    agent = agent.tool_arc(tool);
}

// Use in orchestration
let result = supervisor::orchestrate_custom_agents(
    agents.build(),
    task
).await?;
```

**Prerequisites**:
- `export OPENAI_API_KEY=your_key`
- `export BRAVE_API_KEY=your_key`
- MCP server installed

**Key Concepts**:
- Dynamic tool discovery
- Agent tool integration
- Supervisor orchestration
- Real web search

**Run**: `cargo run --example supervisor_dynamic_mcp_tools`

### supervisor_mcp_research_pipeline.rs
**Difficulty**: Advanced
**Purpose**: Research pipeline with MCP integration

Multi-agent research pipeline using MCP Brave Search.

**Pipeline**: Research Agent → Analysis Agent → Reporting Agent

**Prerequisites**:
- OpenAI API key
- Brave API key
- Brave Search MCP server

**Key Concepts**:
- Multi-agent pipelines
- MCP tool integration
- Real-time web research
- Result aggregation

**Run**: `cargo run --example supervisor_mcp_research_pipeline`

## Pipeline Examples

### supervisor_database_pipeline_compact.rs
**Difficulty**: Intermediate
**Purpose**: Database analysis pipeline (condensed version)

Demonstrates a three-agent pipeline for database analysis.

**Pipeline**: Database Agent → Analysis Agent → Reporting Agent

```rust
let database_agent = AgentBuilder::new("database_agent")
    .tool(QueryRevenueTool::new())
    .tool(QueryRegionsTool::new());

let agents = AgentCollection::new()
    .add(database_agent)
    .add(analysis_agent)
    .add(reporting_agent);

supervisor::orchestrate_custom_agents(agents.build(), task).await?;
```

**Key Concepts**:
- Multi-agent pipelines
- Custom tools
- Data flow between agents
- Compact example for blog posts

**Run**: `cargo run --example supervisor_database_pipeline_compact`

### supervisor_database_pipeline.rs
**Difficulty**: Advanced
**Purpose**: Full database analysis pipeline

Complete version with more tools and data.

**Run**: `cargo run --example supervisor_database_pipeline`

### supervisor_database_validation_compact.rs
**Difficulty**: Advanced
**Purpose**: Pipeline with validation framework

Shows handoff validation between agents.

```rust
coordinator.register_contract(
    "database_agent_handoff".to_string(),
    HandoffContract {
        from_agent: "database_agent".to_string(),
        to_agent: Some("analysis_agent".to_string()),
        schema: OutputSchema {
            required_fields: vec!["data".to_string()],
            validation_rules: vec![/* rules */],
        },
    },
);
```

**Key Concepts**:
- Handoff contracts
- Schema validation
- Quality gates
- Pipeline integrity

**Run**: `cargo run --example supervisor_database_validation_compact`

### supervisor_database_pipeline_with_validation.rs
**Difficulty**: Advanced
**Purpose**: Full pipeline with validation

Complete version with comprehensive validation.

**Run**: `cargo run --example supervisor_database_pipeline_with_validation`

## Validation Examples

### validation_demo.rs
**Difficulty**: Intermediate
**Purpose**: Handoff validation framework

Demonstrates the validation system in isolation.

**Key Concepts**:
- Validation rules
- Schema contracts
- Type checking
- Range validation

**Run**: `cargo run --example validation_demo`

### handoff_validation_example.rs
**Difficulty**: Intermediate
**Purpose**: Agent handoff validation

Shows validation at agent boundaries.

**Run**: `cargo run --example handoff_validation_example`

## Session Examples

### session_usage.rs
**Difficulty**: Intermediate
**Purpose**: Session management for conversations

Demonstrates maintaining conversational state.

**Key Concepts**:
- Session creation
- Message history
- Context management

**Run**: `cargo run --example session_usage`

### interactive_session.rs
**Difficulty**: Intermediate
**Purpose**: Interactive conversational interface

Build an interactive chat interface.

**Run**: `cargo run --example interactive_session`

## Advanced Examples

### supervisor_code_review_pipeline.rs
**Difficulty**: Advanced
**Purpose**: Code review workflow

Multi-agent code review pipeline.

**Pipeline**: Analyzer → Security Checker → Documentation Reviewer → Reporter

**Run**: `cargo run --example supervisor_code_review_pipeline`

### router_usage.rs
**Difficulty**: Advanced
**Purpose**: Direct message router usage

Low-level actor system interaction.

**Run**: `cargo run --example router_usage`

### router_custom_agents.rs
**Difficulty**: Advanced
**Purpose**: Custom agents with direct router

Shows how to work with the router directly.

**Run**: `cargo run --example router_custom_agents`

## Example Categories Summary

### By Difficulty

**Beginner**:
- simple_usage.rs
- mcp_simple_demo.rs
- simplified_agent_builder.rs

**Intermediate**:
- advanced_usage.rs
- batch_processing.rs
- agent_usage.rs
- supervisor_usage.rs
- tool_with_macro.rs
- tool_function_style.rs
- mcp_discover_tools.rs
- supervisor_database_pipeline_compact.rs
- validation_demo.rs
- session_usage.rs

**Advanced**:
- supervisor_custom_agents.rs
- advanced_tool_macro.rs
- supervisor_with_custom_tools.rs
- supervisor_dynamic_mcp_tools.rs
- supervisor_mcp_research_pipeline.rs
- supervisor_database_pipeline.rs
- supervisor_database_validation_compact.rs
- supervisor_code_review_pipeline.rs
- router_usage.rs

### By Feature

**Basic LLM Operations**:
- simple_usage.rs
- advanced_usage.rs
- batch_processing.rs

**Agent System**:
- agent_usage.rs
- supervisor_usage.rs
- supervisor_custom_agents.rs
- agent_introspection.rs

**Tools**:
- tool_with_macro.rs
- tool_function_style.rs
- advanced_tool_macro.rs
- supervisor_with_custom_tools.rs

**MCP Integration**:
- mcp_simple_demo.rs
- mcp_discover_tools.rs
- supervisor_dynamic_mcp_tools.rs
- supervisor_mcp_research_pipeline.rs

**Pipelines**:
- supervisor_database_pipeline_compact.rs
- supervisor_database_pipeline.rs
- supervisor_code_review_pipeline.rs

**Validation**:
- validation_demo.rs
- handoff_validation_example.rs
- supervisor_database_validation_compact.rs

## Common Patterns

### Pattern 1: Simple Agent Creation

```rust
use actorus::{init, AgentBuilder};

init().await?;

let agent = AgentBuilder::new("my_agent")
    .description("Agent description")
    .system_prompt("System prompt")
    .build();
```

### Pattern 2: MCP Tool Discovery

```rust
use actorus::core::mcp::discover_mcp_tools;

let tools = discover_mcp_tools(
    "npx",
    vec!["-y", "@modelcontextprotocol/server-name"]
).await?;

let mut agent = AgentBuilder::new("agent");
for tool in tools {
    agent = agent.tool_arc(tool);
}
```

### Pattern 3: Multi-Agent Pipeline

```rust
use actorus::{AgentBuilder, AgentCollection, supervisor};

let agents = AgentCollection::new()
    .add(agent1)
    .add(agent2)
    .add(agent3);

let result = supervisor::orchestrate_custom_agents(
    agents.build(),
    "Task description"
).await?;
```

### Pattern 4: Custom Tools

```rust
#[tool_fn(
    name = "tool_name",
    description = "Tool description"
)]
async fn my_tool(_param: String) -> Result<String> {
    Ok("result".to_string())
}

let agent = AgentBuilder::new("agent")
    .tool(MyToolTool::new());
```

## Troubleshooting

### Common Issues

1. **Missing API Key**
   ```
   Error: OPENAI_API_KEY environment variable not set
   ```
   Solution: `export OPENAI_API_KEY=your_key`

2. **MCP Server Not Found**
   ```
   Error: EOF while parsing a value
   ```
   Solution: Install MCP server: `npm install -g @modelcontextprotocol/server-name`

3. **Brave API Key Missing**
   ```
   WARNING: BRAVE_API_KEY not set
   ```
   Solution: `export BRAVE_API_KEY=your_key`

## Next Steps

After exploring examples:

1. Read [ACTOR_AGENTS.md](./ACTOR_AGENTS.md) for architecture details
2. Check [ARCHITECTURE.md](./ARCHITECTURE.md) for system overview
3. Review [CONTRIBUTING.md](./CONTRIBUTING.md) to contribute
4. Build your own agent system!

## Example Request

Missing an example? Open an issue or submit a PR!
