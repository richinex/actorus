# Quick Start - Autonomous Agent

## Setup

1. Set your OpenAI API key:
```bash
export OPENAI_API_KEY=your_key_here
```

2. Run the agent example:
```bash
cargo run --example agent_usage
```

## Basic Usage

```rust
use llm_fusion::{init, agent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize once
    init().await?;

    // Run autonomous task
    let result = agent::run_task("Create hello.txt with 'Hello, World!'").await?;

    if result.success {
        println!("Result: {}", result.result);

        // See what the agent did
        for step in result.steps {
            println!("Step {}: {}", step.iteration, step.thought);
        }
    }

    Ok(())
}
```

## Available Tools

The agent can use these tools automatically:

- **execute_shell**: Run shell commands
- **read_file**: Read file contents
- **write_file**: Create/update files
- **http_request**: Make HTTP requests

## Example Tasks

### File Operations
```rust
agent::run_task("Create a file called notes.txt with today's date").await?;
agent::run_task("Read the contents of README.md and count the lines").await?;
```

### Shell Commands
```rust
agent::run_task("List all Rust files in this directory").await?;
agent::run_task("Check the current git status").await?;
```

### HTTP Requests
```rust
agent::run_task("Fetch data from https://api.github.com/repos/rust-lang/rust").await?;
```

### Multi-Step Tasks
```rust
agent::run_task(
    "Create a report of all .rs files with their line counts and save to report.txt"
).await?;
```

## Control Iterations

```rust
// Default: 10 iterations
agent::run_task("task").await?;

// Custom: 20 iterations
agent::run_task_with_iterations("complex task", 20).await?;
```

## Inspect Agent Reasoning

```rust
let result = agent::run_task("task").await?;

for step in result.steps {
    println!("Iteration {}", step.iteration);
    println!("  Thought: {}", step.thought);

    if let Some(action) = step.action {
        println!("  Tool: {}", action);
    }

    if let Some(obs) = step.observation {
        println!("  Result: {}", obs);
    }
}
```

## Error Handling

```rust
let result = agent::run_task("task").await?;

if !result.success {
    if let Some(error) = result.error {
        println!("Agent failed: {}", error);
    }
    // Still have access to partial steps
    for step in result.steps {
        println!("Completed step: {}", step.thought);
    }
}
```

## Agent Lifecycle

```rust
// Stop the agent actor gracefully
agent::stop().await?;

// The agent will finish current task and shutdown
// Useful for cleanup or reconfiguration
```

## Next Steps

See `IMPLEMENTATION_SUMMARY.md` for:
- Architecture details
- Adding custom tools
- Multi-agent coordination patterns
- Production deployment guidance
