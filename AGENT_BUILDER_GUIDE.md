# Agent Builder Guide

## Overview

The `AgentBuilder` API provides a simplified, fluent interface for creating custom specialized agents. This guide shows you how to build agents more easily than the previous tuple-based approach.

## Quick Comparison

### Before (Old Way)

```rust
// Manual Arc wrapping, verbose tuples
let data_tools: Vec<Arc<dyn Tool>> = vec![
    Arc::new(AddItemTool::new()),
    Arc::new(SearchItemsTool::new()),
    Arc::new(CountItemsTool::new()),
];

let data_agent_config = (
    "data_agent".to_string(),
    "Manages inventory data - can add items, search by category, and count totals".to_string(),
    "You are a data management specialist. Use your inventory tools to manage and query data.".to_string(),
    data_tools,
);
```

### After (New Way)

```rust
// Clean, fluent API - no manual Arc wrapping
let data_agent = AgentBuilder::new("data_agent")
    .description("Manages inventory data - can add items, search by category, and count totals")
    .system_prompt("You are a data management specialist. Use your inventory tools to manage and query data.")
    .tool(AddItemTool::new())
    .tool(SearchItemsTool::new())
    .tool(CountItemsTool::new());
```

## Basic Usage

### Step 1: Import Required Types

```rust
use llm_fusion::{AgentBuilder, AgentCollection, init, supervisor};
use llm_fusion::tool_fn;
```

### Step 2: Define Your Tools

Use the `#[tool_fn]` macro to create custom tools:

```rust
#[tool_fn(
    name = "add_item",
    description = "Add a new item to the inventory"
)]
async fn add_item(name: String, quantity: i64) -> Result<String> {
    Ok(format!("Added {} units of '{}'", quantity, name))
}

#[tool_fn(
    name = "search_items",
    description = "Search for items in the inventory"
)]
async fn search_items(category: String) -> Result<String> {
    Ok(format!("Found items in category '{}'", category))
}
```

### Step 3: Build Agents with AgentBuilder

```rust
let data_agent = AgentBuilder::new("data_agent")
    .description("Manages inventory data")
    .system_prompt("You are a data management specialist")
    .tool(AddItemTool::new())
    .tool(SearchItemsTool::new());
```

### Step 4: Collect Multiple Agents

```rust
let agents = AgentCollection::new()
    .add(data_agent)
    .add(text_agent)
    .add(math_agent);

// Convert to the format needed by supervisor API
let agent_configs = agents.build();
```

### Step 5: Use with Supervisor or Router

```rust
init().await?;

// Option 1: Use with Supervisor (multi-step orchestration)
let result = supervisor::orchestrate_with_custom_agents(
    agent_configs.clone(),
    "Add items to inventory and calculate totals"
).await?;

// Option 2: Use with Router (intent-based routing)
let result = router::route_task_with_custom_agents(
    agent_configs,
    "Add an item to the inventory"
).await?;
```

## API Reference

### AgentBuilder

#### Creating a Builder

```rust
AgentBuilder::new(name: impl Into<String>) -> Self
```

Creates a new agent builder with the given name.

#### Builder Methods

All methods return `Self` for method chaining:

```rust
// Set agent description (used by routers/supervisors)
.description(description: impl Into<String>) -> Self

// Set system prompt (guides agent behavior)
.system_prompt(prompt: impl Into<String>) -> Self

// Add a single tool (automatically Arc-wrapped)
.tool<T: Tool + 'static>(tool: T) -> Self

// Add multiple tools at once
.tools<T: Tool + 'static>(tools: Vec<T>) -> Self

// Add a pre-wrapped Arc<dyn Tool>
.tool_arc(tool: Arc<dyn Tool>) -> Self

// Build the agent configuration tuple
.build() -> (String, String, String, Vec<Arc<dyn Tool>>)
```

#### Query Methods

```rust
// Get the agent name
.name() -> &str

// Get the number of tools
.tool_count() -> usize
```

### AgentCollection

#### Creating a Collection

```rust
AgentCollection::new() -> Self
```

Creates an empty agent collection.

#### Collection Methods

```rust
// Add an agent from a builder
.add(builder: AgentBuilder) -> Self

// Add a pre-built agent config
.add_config(config: (String, String, String, Vec<Arc<dyn Tool>>)) -> Self

// Build into vector of configs
.build() -> Vec<(String, String, String, Vec<Arc<dyn Tool>>)>

// Get number of agents
.len() -> usize

// Check if empty
.is_empty() -> bool

// List all agents (name, description)
.list_agents() -> Vec<(&str, &str)>
```

## Advanced Patterns

### Default Values

If you don't provide description or system_prompt, sensible defaults are generated:

```rust
let agent = AgentBuilder::new("my_agent")
    .tool(MyTool::new())
    .build();

// Description: "Specialized agent: my_agent"
// System Prompt: "You are a specialized agent named my_agent. Use your available tools to complete tasks."
```

### Adding Multiple Tools

Three ways to add tools:

```rust
// 1. One at a time (recommended for clarity)
let agent = AgentBuilder::new("agent")
    .tool(Tool1::new())
    .tool(Tool2::new())
    .tool(Tool3::new());

// 2. Multiple at once
let tools = vec![Tool1::new(), Tool2::new()];
let agent = AgentBuilder::new("agent")
    .tools(tools);

// 3. Pre-wrapped Arc tools
let tool_arc = Arc::new(Tool1::new());
let agent = AgentBuilder::new("agent")
    .tool_arc(tool_arc);
```

### Inspecting Before Building

```rust
let builder = AgentBuilder::new("my_agent")
    .tool(Tool1::new())
    .tool(Tool2::new());

println!("Agent: {}", builder.name());
println!("Tools: {}", builder.tool_count());

let config = builder.build();
```

### Managing Multiple Agents

```rust
// Create agents
let agent1 = AgentBuilder::new("agent1")
    .description("First agent")
    .tool(Tool1::new());

let agent2 = AgentBuilder::new("agent2")
    .description("Second agent")
    .tool(Tool2::new());

// Collect them
let collection = AgentCollection::new()
    .add(agent1)
    .add(agent2);

// Inspect before using
println!("Total agents: {}", collection.len());
for (name, desc) in collection.list_agents() {
    println!("  - {}: {}", name, desc);
}

// Build and use
let configs = collection.build();
supervisor::orchestrate_with_custom_agents(configs, task).await?;
```

## Complete Example

```rust
use anyhow::Result;
use llm_fusion::{init, supervisor, AgentBuilder, AgentCollection, tool_fn};

// Define custom tools
#[tool_fn(name = "add", description = "Add two numbers")]
async fn add(a: i64, b: i64) -> Result<String> {
    Ok(format!("{} + {} = {}", a, b, a + b))
}

#[tool_fn(name = "multiply", description = "Multiply two numbers")]
async fn multiply(a: i64, b: i64) -> Result<String> {
    Ok(format!("{} * {} = {}", a, b, a * b))
}

#[tool_fn(name = "format_text", description = "Format text")]
async fn format_text(text: String, style: String) -> Result<String> {
    match style.as_str() {
        "upper" => Ok(text.to_uppercase()),
        "lower" => Ok(text.to_lowercase()),
        _ => Ok(text)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init().await?;

    // Build specialized agents
    let math_agent = AgentBuilder::new("math_agent")
        .description("Performs mathematical calculations")
        .system_prompt("You are a math specialist. Use calculation tools.")
        .tool(AddTool::new())
        .tool(MultiplyTool::new());

    let text_agent = AgentBuilder::new("text_agent")
        .description("Processes text")
        .system_prompt("You are a text processing specialist.")
        .tool(FormatTextTool::new());

    // Collect agents
    let agents = AgentCollection::new()
        .add(math_agent)
        .add(text_agent);

    println!("Created {} agents", agents.len());

    // Use with supervisor
    let result = supervisor::orchestrate_with_custom_agents(
        agents.build(),
        "Add 5 and 3, multiply the result by 2, then format the answer as uppercase"
    ).await?;

    println!("Success: {}", result.success);
    println!("Result: {}", result.result);

    Ok(())
}
```

## Benefits

### 1. Less Boilerplate
No manual `Arc::new()` calls, no `.to_string()` everywhere, cleaner code.

### 2. Type Safety
All validation happens at compile time. If it builds, it works.

### 3. Clear Intent
Method names make the purpose obvious: `.description()`, `.system_prompt()`, `.tool()`.

### 4. Fluent API
Chain methods naturally without intermediate variables.

### 5. Default Values
Don't specify optional fields if defaults work for you.

### 6. Easy Management
`AgentCollection` makes it easy to work with multiple agents as a group.

## Integration with Existing APIs

The `AgentBuilder` produces the same tuple format used by existing supervisor APIs:

```rust
// These are equivalent:

// Old way
let config = (
    "agent".to_string(),
    "description".to_string(),
    "prompt".to_string(),
    vec![Arc::new(tool)]
);

// New way
let config = AgentBuilder::new("agent")
    .description("description")
    .system_prompt("prompt")
    .tool(tool)
    .build();

// Both can be used with:
supervisor::orchestrate_with_custom_agents(vec![config], task).await?;
```

## Migration Guide

If you have existing agent creation code, here's how to migrate:

### Step 1: Remove Manual Vec Creation

```rust
// Before
let tools: Vec<Arc<dyn Tool>> = vec![
    Arc::new(Tool1::new()),
    Arc::new(Tool2::new()),
];

// After - handled by builder
// (just use .tool() methods)
```

### Step 2: Replace Tuple with Builder

```rust
// Before
let config = (
    "agent_name".to_string(),
    "description".to_string(),
    "system_prompt".to_string(),
    tools,
);

// After
let config = AgentBuilder::new("agent_name")
    .description("description")
    .system_prompt("system_prompt")
    .tool(Tool1::new())
    .tool(Tool2::new())
    .build();
```

### Step 3: Use AgentCollection (Optional)

```rust
// Before
let configs = vec![agent1_config, agent2_config, agent3_config];

// After
let configs = AgentCollection::new()
    .add(agent1_builder)
    .add(agent2_builder)
    .add(agent3_builder)
    .build();
```

## Examples

See these files for complete working examples:

- `examples/simplified_agent_builder.rs` - Full example with multiple agents
- `examples/supervisor_custom_agents.rs` - Supervisor with custom agents
- `examples/router_custom_agents.rs` - Router with custom agents (NEW)

Run the examples:

```bash
# AgentBuilder demonstration
cargo run --example simplified_agent_builder

# Supervisor with custom agents
cargo run --example supervisor_custom_agents

# Router with custom agents
cargo run --example router_custom_agents
```

## Best Practices

### 1. Name Agents Descriptively

```rust
// Good
AgentBuilder::new("inventory_management_agent")
AgentBuilder::new("text_processor")

// Avoid
AgentBuilder::new("agent1")
AgentBuilder::new("a")
```

### 2. Provide Clear Descriptions

Descriptions help routers and supervisors choose the right agent:

```rust
.description("Manages inventory: add items, search by category, count totals")
```

### 3. Write Specific System Prompts

System prompts guide agent behavior:

```rust
.system_prompt("You are a math specialist. Use your calculation tools to solve problems. Always show your work.")
```

### 4. Group Related Tools

Put tools that work together in the same agent:

```rust
// Good - related file operations
let file_agent = AgentBuilder::new("file_agent")
    .tool(ReadFileTool::new())
    .tool(WriteFileTool::new())
    .tool(DeleteFileTool::new());

// Avoid - unrelated tools
let mixed_agent = AgentBuilder::new("mixed_agent")
    .tool(ReadFileTool::new())
    .tool(CalculateTool::new())
    .tool(HttpRequestTool::new());
```

### 5. Test Agent Configurations

Before using agents in production, verify they work:

```rust
let agent = AgentBuilder::new("test_agent")
    .tool(MyTool::new());

assert_eq!(agent.name(), "test_agent");
assert_eq!(agent.tool_count(), 1);
```

## Troubleshooting

### "Tool does not implement `Tool` trait"

Make sure your tool struct implements the `Tool` trait, either manually or via `#[tool_fn]` macro.

### "Cannot move out of borrowed content"

Use `.clone()` if you need to use the same tool in multiple agents:

```rust
let tool = MyTool::new();
let agent1 = AgentBuilder::new("a1").tool(tool.clone());
let agent2 = AgentBuilder::new("a2").tool(tool.clone());
```

Or better, just create new instances:

```rust
let agent1 = AgentBuilder::new("a1").tool(MyTool::new());
let agent2 = AgentBuilder::new("a2").tool(MyTool::new());
```

### "AgentBuilder not found"

Make sure you've imported it:

```rust
use llm_fusion::AgentBuilder;
```

## Future Enhancements

Potential additions to the AgentBuilder API:

- Agent metadata (version, author, tags)
- Validation hooks
- Tool dependencies
- Agent presets/templates
- Configuration from files
- Agent registry/marketplace

## Feedback

The AgentBuilder API is designed to be intuitive and easy to use. If you have suggestions for improvements, please open an issue or discussion on the project repository.
