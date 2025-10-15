# Multi-Agent System Usage Guide

## Quick Start

### Prerequisites
```bash
export OPENAI_API_KEY=your_api_key_here
```

### Three Usage Patterns

#### 1. Simple Agent (Original - Fastest)
Use when you need general-purpose autonomous task execution:

```rust
use llm_fusion::{init, agent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init().await?;

    let result = agent::run_task(
        "List all files in the current directory"
    ).await?;

    println!("Success: {}", result.success);
    println!("Result: {}", result.result);

    Ok(())
}
```

**Best for**: Simple tasks, when you don't care which tools are used

#### 2. Router Agent (Intent-Based Routing)
Use when you have clear single-domain tasks:

```rust
use llm_fusion::{init, router};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init().await?;

    // Automatically routes to appropriate specialized agent
    let result = router::route_task(
        "Create a file named hello.txt with 'Hello, World!'"
    ).await?;

    println!("Success: {}", result.success);
    println!("Result: {}", result.result);

    Ok(())
}
```

**Best for**:
- File operations (routes to file_ops_agent)
- Shell commands (routes to shell_agent)
- Web requests (routes to web_agent)

**How it works**:
1. LLM classifies the intent
2. Routes to ONE specialized agent
3. Agent executes task with domain-specific tools
4. Returns result (one-way ticket)

#### 3. Supervisor Agent (Multi-Step Orchestration)
Use when you have complex tasks spanning multiple domains:

```rust
use llm_fusion::{init, supervisor};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init().await?;

    // Supervisor orchestrates multiple agents
    let result = supervisor::orchestrate(
        "List all Rust files in src/, count them, \
         and write the count to rust_count.txt"
    ).await?;

    println!("Success: {}", result.success);
    println!("Result: {}", result.result);
    println!("Steps: {}", result.steps.len());

    Ok(())
}
```

**Best for**:
- Multi-step tasks
- Tasks requiring multiple tool types
- Complex workflows

**How it works**:
1. Supervisor decomposes task into sub-tasks
2. Invokes specialized agents in sequence
3. Can invoke same agent multiple times (return ticket)
4. Combines results into final answer

## API Reference

### agent Module

```rust
// Run a task with default settings (10 max iterations)
pub async fn run_task(task: impl Into<String>) -> Result<AgentResult>

// Run with custom max iterations
pub async fn run_task_with_iterations(
    task: impl Into<String>,
    max_iterations: usize
) -> Result<AgentResult>

// Stop the agent actor
pub async fn stop() -> Result<()>
```

### router Module

```rust
// Route a task to appropriate built-in agent (10 max iterations per agent)
pub async fn route_task(task: impl Into<String>) -> Result<AgentResult>

// Route with custom max iterations
pub async fn route_task_with_iterations(
    task: impl Into<String>,
    max_iterations: usize
) -> Result<AgentResult>

// Route to custom agents built with AgentBuilder (NEW)
pub async fn route_task_with_custom_agents(
    agent_configs: Vec<(String, String, String, Vec<Arc<dyn Tool>>)>,
    task: impl Into<String>
) -> Result<AgentResult>

// Route to custom agents with custom max iterations (NEW)
pub async fn route_task_with_custom_agents_and_iterations(
    agent_configs: Vec<(String, String, String, Vec<Arc<dyn Tool>>)>,
    task: impl Into<String>,
    max_iterations: usize
) -> Result<AgentResult>
```

### supervisor Module

```rust
// Orchestrate a complex task (10 max orchestration steps)
pub async fn orchestrate(task: impl Into<String>) -> Result<AgentResult>

// Orchestrate with custom max steps
pub async fn orchestrate_with_steps(
    task: impl Into<String>,
    max_orchestration_steps: usize
) -> Result<AgentResult>
```

### AgentResult Structure

All three patterns return the same result type:

```rust
pub struct AgentResult {
    pub success: bool,
    pub result: String,
    pub steps: Vec<AgentStepInfo>,
    pub error: Option<String>,
}

pub struct AgentStepInfo {
    pub iteration: usize,
    pub thought: String,
    pub action: Option<String>,
    pub observation: Option<String>,
}
```

## Specialized Agents

The router and supervisor use these specialized agents:

| Agent | Tools | Best For |
|-------|-------|----------|
| file_ops_agent | read_file, write_file | File I/O operations |
| shell_agent | execute_shell | Shell commands, system operations |
| web_agent | http_request | HTTP requests, web scraping |
| general_agent | All tools | Mixed operations, unclear intent |

## Decision Guide: Which Pattern to Use?

### Use Simple Agent When:
- You need quick, general-purpose execution
- You don't care about agent specialization
- Performance is critical (no routing overhead)
- Task is straightforward

### Use Router Agent When:
- Task clearly fits one domain (files, shell, or web)
- You want specialized tool selection
- You need better organization
- Task is single-domain but complex

### Use Supervisor Agent When:
- Task requires multiple steps
- Task spans multiple domains
- You need coordinated execution
- Task requires results from one agent to feed into another

## Custom Agents with Router and Supervisor

You can now build custom specialized agents and use them with both router and supervisor patterns using the AgentBuilder API:

```rust
use llm_fusion::{init, router, supervisor, AgentBuilder, AgentCollection, tool_fn};

// Define custom tools
#[tool_fn(name = "get_weather", description = "Get weather for a location")]
async fn get_weather(location: String) -> Result<String> {
    Ok(format!("Weather in {}: Sunny, 22°C", location))
}

#[tool_fn(name = "send_email", description = "Send an email")]
async fn send_email(to: String, subject: String) -> Result<String> {
    Ok(format!("Email sent to {} with subject '{}'", to, subject))
}

#[tokio::main]
async fn main() -> Result<()> {
    init().await?;

    // Build custom agents with AgentBuilder
    let weather_agent = AgentBuilder::new("weather_agent")
        .description("Handles weather queries")
        .system_prompt("You are a weather specialist.")
        .tool(GetWeatherTool::new());

    let email_agent = AgentBuilder::new("email_agent")
        .description("Handles email operations")
        .system_prompt("You are an email specialist.")
        .tool(SendEmailTool::new());

    // Collect agents
    let agents = AgentCollection::new()
        .add(weather_agent)
        .add(email_agent);

    let agent_configs = agents.build();

    // Use with router (LLM picks ONE agent based on intent)
    let result = router::route_task_with_custom_agents(
        agent_configs.clone(),
        "What's the weather like in New York?"
    ).await?;
    println!("Router result: {}", result.result);

    // Use with supervisor (orchestrates MULTIPLE agents)
    let result = supervisor::orchestrate_with_custom_agents(
        agent_configs,
        "Check weather in Paris and email it to john@example.com"
    ).await?;
    println!("Supervisor result: {}", result.result);

    Ok(())
}
```

See `AGENT_BUILDER_GUIDE.md` for complete AgentBuilder documentation and `examples/router_custom_agents.rs` for a full working example.

## Examples

### Example 1: File Operations (Router)
```rust
// Router automatically selects file_ops_agent
let result = router::route_task(
    "Read config.toml and write its contents to backup.toml"
).await?;
```

### Example 2: Shell Commands (Router)
```rust
// Router automatically selects shell_agent
let result = router::route_task(
    "List all .rs files in the src directory"
).await?;
```

### Example 3: Web Request (Router)
```rust
// Router automatically selects web_agent
let result = router::route_task(
    "Fetch JSON from https://api.example.com/data"
).await?;
```

### Example 4: Multi-Step (Supervisor)
```rust
// Supervisor coordinates shell_agent → file_ops_agent
let result = supervisor::orchestrate(
    "Run 'ls -la', capture the output, and save it to directory_listing.txt"
).await?;
```

### Example 5: Cross-Domain (Supervisor)
```rust
// Supervisor coordinates web_agent → file_ops_agent → shell_agent
let result = supervisor::orchestrate(
    "Fetch data from https://httpbin.org/json, \
     save it to data.json, \
     then count the lines in the file"
).await?;
```

## Running the Examples

```bash
# Simple agent (improved completion detection)
RUST_LOG=info cargo run --example simple_agent_test

# Router pattern with built-in agents
RUST_LOG=info cargo run --example router_usage

# Router pattern with custom agents (NEW)
RUST_LOG=info cargo run --example router_custom_agents

# Supervisor pattern with built-in agents
RUST_LOG=info cargo run --example supervisor_usage

# Supervisor pattern with custom agents
RUST_LOG=info cargo run --example supervisor_custom_agents

# AgentBuilder demonstration
RUST_LOG=info cargo run --example simplified_agent_builder
```

## Logging

Control verbosity with RUST_LOG:

```bash
# Info level - see major decisions
RUST_LOG=info cargo run --example router_usage

# Debug level - see all reasoning steps
RUST_LOG=debug cargo run --example supervisor_usage

# Trace level - see everything including heartbeats
RUST_LOG=trace cargo run --example simple_agent_test
```

## Performance Considerations

| Pattern | LLM Calls | Best Case | Worst Case |
|---------|-----------|-----------|------------|
| Simple Agent | 1-10 | 2 steps | 10 steps |
| Router | 1 + (1-10) | 3 steps total | 11 steps total |
| Supervisor | 1-10 orchestration + agents | 3 steps | 50+ steps |

**Recommendation**: Start with Simple Agent, upgrade to Router for better organization, use Supervisor only for genuinely complex tasks.

## Error Handling

All functions return `Result<AgentResult>`:

```rust
let result = router::route_task("Invalid task").await?;

if !result.success {
    if let Some(error) = result.error {
        eprintln!("Task failed: {}", error);
    }
}
```

## Actor System Integration

All three patterns benefit from the underlying actor system:
- Automatic fault recovery
- Heartbeat monitoring
- Clean shutdown
- Message passing reliability

The actor system is completely transparent to the user.

## Advanced Usage

### Custom Iterations
```rust
// Agent: Allow more iterations for complex reasoning
let result = agent::run_task_with_iterations(task, 20).await?;

// Router: Control per-agent iterations
let result = router::route_task_with_iterations(task, 15).await?;

// Supervisor: Control orchestration steps
let result = supervisor::orchestrate_with_steps(task, 15).await?;
```

### Inspecting Steps
```rust
let result = supervisor::orchestrate(task).await?;

println!("Agent took {} steps:", result.steps.len());
for (i, step) in result.steps.iter().enumerate() {
    println!("Step {}: {}", i + 1, step.thought);
    if let Some(action) = &step.action {
        println!("  Action: {}", action);
    }
    if let Some(obs) = &step.observation {
        println!("  Result: {}", obs);
    }
}
```

## Tips & Best Practices

1. **Start Simple**: Use simple agent first, upgrade if needed
2. **Be Specific**: Clear task descriptions get better results
3. **Check Steps**: Inspect `result.steps` to understand agent reasoning
4. **Use Logging**: Enable INFO logging to see routing decisions
5. **Handle Errors**: Always check `result.success` and `result.error`
6. **Reasonable Limits**: Don't set max_iterations too high (10-20 is good)
7. **Task Clarity**: More specific tasks = better agent selection

## Troubleshooting

### Router picks wrong agent
- Make your task description more specific
- Check logs to see routing decision reasoning
- Consider using simple agent if task is ambiguous

### Supervisor doesn't complete
- Check max_orchestration_steps (may need more)
- Review logs to see where it's stuck
- Break task into smaller pieces

### Agent repeats actions
- This should be fixed with improved prompts
- If still happening, check agent logs
- May need to adjust max_iterations

## Next Steps

See `IMPLEMENTATION_COMPLETE.md` for:
- Architecture details
- Implementation status
- Files modified/created
- Design decisions

See `BOOKIDEAS.md` for:
- Theoretical background
- Router pattern (Section 12.2)
- Supervisor pattern (Section 12.3)
