# Actorus

A Rust library providing a simple async API with actor-based reliability for LLM interactions. Built on an actor pattern architecture for fault tolerance, concurrency, and distributed agent systems.

## Overview

Actorus combines the simplicity of async/await with the robustness of the actor model. It provides:

- Simple async API for LLM interactions
- Actor-based system for fault tolerance
- Multi-agent orchestration with supervisor pattern
- Tool integration for extending agent capabilities
- MCP (Model Context Protocol) integration for external services
- Session management for conversational AI
- Validation framework for agent handoffs

## Features

### Core Features

- **Async-First Design**: Built on Tokio for high-performance async operations
- **Actor-Based Architecture**: Each component is an isolated actor communicating via message passing
- **Fault Tolerance**: Automatic actor restart, health monitoring, and graceful degradation
- **Message Routing**: Centralized router for efficient actor communication
- **Type Safety**: Strongly typed messages and responses

### Agent System

- **Specialized Agents**: Create agents with specific roles and tools
- **Supervisor Pattern**: LLM-powered orchestration for complex multi-step tasks
- **Agent Builder API**: Fluent interface for agent configuration
- **Tool Integration**: Extensible tool system with macro support
- **Agent Introspection**: Query agent capabilities and state

### MCP Integration

- **Dynamic Tool Discovery**: Automatically discover tools from any MCP server
- **Plug-and-Play**: No code changes needed to add new MCP servers
- **External Services**: Integrate web search, filesystem, GitHub, and more
- **Tool Wrapping**: Automatic conversion of MCP tools to agent tools

### Validation Framework

- **Handoff Contracts**: Define schema and validation rules between agents
- **Quality Gates**: Ensure data quality at agent boundaries
- **Schema Validation**: Type checking, range validation, enum validation
- **Pipeline Integrity**: Maintain data consistency across agent workflows

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
actorus = "0.1.0"
```

### Basic Usage

```rust
use actorus::{init, generate_text};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the system
    init().await?;

    // Generate text
    let response = generate_text("Explain Rust ownership", None).await?;
    println!("{}", response);

    Ok(())
}
```

### Agent Usage

```rust
use actorus::{init, AgentBuilder, supervisor};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init().await?;

    // Create specialized agent
    let research_agent = AgentBuilder::new("researcher")
        .description("Research specialist")
        .system_prompt("You are a research expert.")
        .build();

    // Use supervisor to orchestrate
    let result = supervisor::orchestrate(
        vec![research_agent],
        "Research Rust async patterns"
    ).await?;

    println!("{}", result.result);
    Ok(())
}
```

### MCP Integration

```rust
use actorus::core::mcp::discover_mcp_tools;
use actorus::{init, AgentBuilder, supervisor, AgentCollection};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init().await?;

    // Discover tools from MCP server
    let tools = discover_mcp_tools(
        "npx",
        vec!["-y", "@modelcontextprotocol/server-brave-search"]
    ).await?;

    // Create agent with MCP tools
    let mut agent = AgentBuilder::new("research_agent")
        .description("Research agent with web search");

    for tool in tools {
        agent = agent.tool_arc(tool);
    }

    // Use in orchestration
    let agents = AgentCollection::new().add(agent);
    let result = supervisor::orchestrate_custom_agents(
        agents.build(),
        "Research latest AI developments"
    ).await?;

    Ok(())
}
```

## Architecture

Actorus is built on the actor pattern:

```
┌─────────────────────────────────────────────────────────┐
│                    Message Router                        │
│  (Central hub for all actor communication)              │
└─────────────────────────────────────────────────────────┘
           │              │              │
           ▼              ▼              ▼
    ┌──────────┐   ┌──────────┐   ┌──────────┐
    │   LLM    │   │  Agent   │   │   MCP    │
    │  Actor   │   │  Actor   │   │  Actor   │
    └──────────┘   └──────────┘   └──────────┘
```

### Key Components

- **Message Router**: Centralized actor communication hub
- **LLM Actor**: Handles OpenAI API interactions
- **Agent Actor**: Manages specialized agent execution
- **Supervisor Agent**: Orchestrates multi-agent workflows
- **MCP Actor**: Interfaces with external MCP servers
- **Health Monitor**: Tracks actor health and enables recovery

See [ACTOR_AGENTS.md](./ACTOR_AGENTS.md) for detailed architecture documentation.

## Examples

The `examples/` directory contains comprehensive examples:

### Basic Examples
- `simple_usage.rs` - Basic text generation
- `advanced_usage.rs` - Streaming, retries, fallback
- `batch_processing.rs` - Process multiple prompts

### Agent Examples
- `agent_usage.rs` - Specialized agent creation
- `supervisor_usage.rs` - Multi-agent orchestration
- `agent_introspection.rs` - Query agent capabilities

### Tool Examples
- `tool_with_macro.rs` - Create tools with macros
- `tool_function_style.rs` - Function-style tool definition
- `supervisor_with_custom_tools.rs` - Agents with custom tools

### MCP Examples
- `mcp_discover_tools.rs` - Discover MCP server tools
- `mcp_simple_demo.rs` - Basic MCP usage
- `supervisor_dynamic_mcp_tools.rs` - Dynamic MCP integration
- `supervisor_mcp_research_pipeline.rs` - MCP research workflow

### Pipeline Examples
- `supervisor_database_pipeline_compact.rs` - Database analysis pipeline
- `supervisor_database_validation_compact.rs` - Pipeline with validation
- `validation_demo.rs` - Handoff validation framework

Run any example:
```bash
cargo run --example simple_usage
cargo run --example mcp_discover_tools
```

## Configuration

Set environment variables:

```bash
# Required
export OPENAI_API_KEY=your_api_key

# Optional for MCP examples
export BRAVE_API_KEY=your_brave_api_key
```

Create `.env` file:
```env
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4
OPENAI_TEMPERATURE=0.7
```

## Documentation

- [ACTOR_AGENTS.md](./ACTOR_AGENTS.md) - Detailed actor pattern documentation
- [EXAMPLES.md](./EXAMPLES.md) - Comprehensive example guide
- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture overview
- [CONTRIBUTING.md](./CONTRIBUTING.md) - Contribution guidelines

## Key Concepts

### Actor Pattern

Each component is an independent actor:
- Isolated state (no shared memory)
- Message-based communication
- Automatic restart on failure
- Concurrent execution

### Supervisor Pattern

LLM-powered task orchestration:
- Breaks complex tasks into sub-goals
- Selects appropriate agents for each sub-goal
- Aggregates results into final output
- Handles failures and retries

### MCP Integration

Model Context Protocol support:
- Dynamic tool discovery from any MCP server
- Automatic tool wrapping
- Plug-and-play architecture
- External service integration

### Validation Framework

Agent handoff validation:
- Schema contracts between agents
- Type and range validation
- Quality gates
- Pipeline integrity checks

## Requirements

- Rust 1.70+
- OpenAI API key
- Optional: MCP servers (for MCP examples)

## License

MIT

## Author

Richard Chukwu

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## Acknowledgments

Built with inspiration from:
- Actor model patterns in Erlang/Elixir
- OpenAI API and function calling
- Model Context Protocol (MCP)
- Rust async ecosystem (Tokio)
