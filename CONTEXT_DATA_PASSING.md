# Context-Based Data Passing Between Agents

This document explains the structured context-based approach for passing data between agents in multi-agent pipelines.

## Overview

Instead of embedding JSON data in task description strings (prompt engineering), agents can now receive structured context data programmatically. This provides type-safe data passing between agents in supervisor-orchestrated workflows.

## Architecture

### Information Hiding

The context system follows information hiding principles:
- **Hidden**: Internal JSON serialization, context building logic, prompt formatting
- **Exposed**: Clean interface via `execute_task_with_context()` method
- **Benefit**: Can change context implementation without affecting agent logic

### Key Components

1. **SpecializedAgent**: Accepts context via `execute_task_with_context()`
2. **SupervisorAgent**: Builds context from previous agent results
3. **Context Structure**: `serde_json::Value` containing previous outputs

## Implementation

### 1. Agent Execution with Context

```rust
// In src/actors/specialized_agent.rs

pub async fn execute_task_with_context(
    &self,
    task: &str,
    context: Option<Value>,
    max_iterations: usize,
) -> AgentResponse
```

The context is displayed in the system prompt as formatted JSON:

```
CONTEXT DATA (use this in your tool calls):
```json
{
  "database_agent_output": {
    "data": [...]
  }
}
```

The context contains structured data from previous steps.
You can reference fields from this data when calling tools.
```

### 2. Supervisor Context Building

```rust
// In src/actors/supervisor_agent.rs

// Track results from all agents
let mut agent_results_context: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

// After each agent completes
let result_value = serde_json::from_str::<serde_json::Value>(result)
    .unwrap_or_else(|_| serde_json::Value::String(result.clone()));
agent_results_context.insert(
    format!("{}_output", agent_name),
    result_value
);

// Pass to next agent
let context = if !agent_results_context.is_empty() {
    Some(serde_json::Value::Object(agent_results_context.clone()))
} else {
    None
};

let agent_response = agent.execute_task_with_context(&agent_task, context, max_iterations).await;
```

### 3. Agent Prompts

Agents should be instructed to use context data:

```rust
let analysis_agent = AgentBuilder::new("analysis_agent")
    .system_prompt(
        "You are a business analyst. \
         You will receive CONTEXT DATA containing output from previous agents. \
         Use the JSON data from context (e.g., database_agent_output) and pass it as a JSON STRING to your analysis tools.\
         \n\nFor example:\n\
         - Context has: database_agent_output: {\"data\": [...]}\n\
         - Convert it to a string: JSON.stringify(database_agent_output)\n\
         - Pass to tool: {\"product_json\": \"<stringified JSON here>\"}\n\
         - Tools expect JSON STRING parameters, not JSON objects",
    )
    .tool(AnalyzeProductDataTool::new())
    .return_tool_output(true);
```

## Benefits

### 1. Type Safety
```rust
// Before: Brittle string parsing
task: "Analyze this data: {\"sales\": [...]}"

// After: Structured data
context: Some(json!({
    "database_agent_output": {
        "sales": [...]
    }
}))
```

### 2. Clean Separation
- **Task Description**: What to do
- **Context Data**: Data to work with

### 3. No Prompt Engineering
The supervisor automatically builds and passes context. No need to craft prompts to embed data.

### 4. Automatic Context Building
```rust
// Supervisor automatically:
// 1. Captures each agent's output
// 2. Stores in context with agent name
// 3. Passes accumulated context to next agent
```

### 5. Backward Compatible
```rust
// Still works without context
agent.execute_task(task, max_iterations).await

// Now supports context
agent.execute_task_with_context(task, Some(context), max_iterations).await
```

## Usage Example

### Database Pipeline with Context

```rust
use llm_fusion::actors::agent_builder::AgentBuilder;
use llm_fusion::actors::supervisor_agent::SupervisorAgent;

// Create agents
let database_agent = AgentBuilder::new("database_agent")
    .description("Fetches data from database")
    .system_prompt("You fetch data from the database")
    .tool(FetchProductDataTool::new())
    .return_tool_output(true);  // Return pure JSON

let analysis_agent = AgentBuilder::new("analysis_agent")
    .description("Analyzes data")
    .system_prompt(
        "You will receive CONTEXT DATA containing database_agent_output. \
         Use this data in your analysis tools."
    )
    .tool(AnalyzeProductDataTool::new())
    .return_tool_output(true);

let reporting_agent = AgentBuilder::new("reporting_agent")
    .description("Generates reports")
    .system_prompt(
        "You will receive CONTEXT DATA containing analysis_agent_output. \
         Use this to generate reports."
    )
    .tool(GenerateReportTool::new())
    .return_tool_output(true);

// Create supervisor with pipeline
let supervisor = SupervisorAgent::new(
    "data_pipeline_supervisor",
    "Orchestrates data pipeline",
    Some("Coordinate agents to fetch, analyze, and report on data"),
    settings,
    api_key.clone(),
)
.with_agent(database_agent)
.with_agent(analysis_agent)
.with_agent(reporting_agent)
.with_goal("goal_1", "Complete data pipeline", vec![
    ("database_agent", "Fetch product sales data"),
    ("analysis_agent", "Analyze the product data"),
    ("reporting_agent", "Generate final report"),
]);

// Execute - context automatically passed between agents
let result = supervisor.execute("Run data pipeline").await?;
```

### Context Flow

```
1. database_agent executes
   └─> Returns: {"data": [...]}

2. Supervisor stores in context:
   {
     "database_agent_output": {"data": [...]}
   }

3. analysis_agent receives context
   └─> Sees database_agent_output in system prompt
   └─> Uses it in analysis tools
   └─> Returns: {"insights": [...]}

4. Supervisor updates context:
   {
     "database_agent_output": {"data": [...]},
     "analysis_agent_output": {"insights": [...]}
   }

5. reporting_agent receives context
   └─> Sees both previous outputs
   └─> Generates report using all data
```

## Integration with Existing Features

### 1. return_tool_output Flag

Context works seamlessly with the `return_tool_output` feature:

```rust
let agent = AgentBuilder::new("data_agent")
    .tool(FetchDataTool::new())
    .return_tool_output(true);  // Returns pure JSON from tool
```

When `return_tool_output = true`:
- Agent returns tool's JSON output directly
- No LLM wrapping or explanation
- Clean JSON for next agent's context

### 2. Handoff Validation

Context data automatically goes through handoff validation:

```rust
supervisor
    .with_handoff_validation(
        "database_agent",
        "analysis_agent",
        Box::new(|output: &str| {
            // Validates the JSON in context
            let data: Value = serde_json::from_str(output)?;
            Ok(format!("Valid data with {} records",
                       data["data"].as_array().unwrap().len()))
        })
    )
```

### 3. Tool Functions

Tools already accept JSON strings via `tool_fn!` macro:

```rust
tool_fn!(
    AnalyzeProductDataTool,
    "analyze_product_data",
    "Analyzes product sales data",
    product_json: String => "JSON string containing product data"
);
```

Agents can pass context data to tools:
```rust
// Context contains: database_agent_output: {...}
// Agent calls: analyze_product_data(product_json: "stringified database_agent_output")
```

## Logging and Debugging

### Context Tracking

```
DEBUG [SupervisorAgent] Passing context with 1 entries to agent 'analysis_agent'
DEBUG [SupervisorAgent] Stored result from 'database_agent' in context
```

### Graceful JSON Parsing

The system handles various LLM response formats:

```rust
// Pure JSON
{"action": "call_tool", "tool": "fetch_data", "args": {...}}

// JSON with explanation (extracted automatically)
Here's what I'll do:
{"action": "call_tool", "tool": "fetch_data", "args": {...}}
This will fetch the data.

// Fallback to thought if no JSON found
Let me think about this...
```

Log levels:
- `debug!`: Initial parse attempt, successful extraction
- `warn!`: Only if complete extraction fails

## Best Practices

### 1. Agent Prompts

Instruct agents to use context data:
```
"You will receive CONTEXT DATA containing previous agent outputs.
 Reference these fields when calling tools."
```

### 2. Tool Design

Design tools to accept JSON strings:
```rust
tool_fn!(
    ProcessDataTool,
    "process_data",
    "Processes data",
    data_json: String => "JSON string with data to process"
);
```

### 3. Return Pure JSON

Use `return_tool_output(true)` for data-producing agents:
```rust
let agent = AgentBuilder::new("data_agent")
    .tool(FetchDataTool::new())
    .return_tool_output(true);  // Clean JSON for next agent
```

### 4. Validation

Add handoff validation to ensure data quality:
```rust
supervisor.with_handoff_validation(
    "producer_agent",
    "consumer_agent",
    Box::new(|output| {
        let data: Value = serde_json::from_str(output)?;
        // Validate structure
        Ok("Validation passed".to_string())
    })
)
```

## Information Hiding Analysis

Following Parnas principles:

### Hidden Design Decisions
1. **Context Serialization**: JSON format, pretty printing
2. **Context Building Logic**: How results are stored and accumulated
3. **Prompt Formatting**: How context is displayed in system prompt
4. **Storage Structure**: Map with "{agent_name}_output" keys

### Stable Interface
```rust
pub async fn execute_task_with_context(
    &self,
    task: &str,
    context: Option<Value>,  // Abstract JSON value
    max_iterations: usize,
) -> AgentResponse
```

### Benefits
- Can change JSON formatting without affecting agents
- Can modify context storage structure without breaking API
- Can optimize context building without touching agent code
- Future: Could add context compression, encryption, validation

## Troubleshooting

### Context Not Available

Check that previous agents completed successfully:
```rust
DEBUG [SupervisorAgent] Stored result from 'agent_name' in context
```

### Tool Parse Errors

Ensure tools expect JSON strings, not objects:
```rust
// Correct
tool_fn!(MyTool, "my_tool", "description", data: String => "JSON string");

// Incorrect
tool_fn!(MyTool, "my_tool", "description", data: Value => "JSON object");
```

### Empty Context

First agent in pipeline will have no context (expected):
```rust
DEBUG [SupervisorAgent] Passing context with 0 entries to agent 'first_agent'
```

## Future Enhancements

1. **Context Filtering**: Pass only relevant context to each agent
2. **Context Validation**: Schema validation for context structure
3. **Context Compression**: For large data pipelines
4. **Context Persistence**: Save/restore context across runs
5. **Context Querying**: Allow agents to query specific context fields

## References

- Example: `examples/supervisor_database_pipeline_with_validation.rs`
- Agent Implementation: `src/actors/specialized_agent.rs:128-148`
- Supervisor Implementation: `src/actors/supervisor_agent.rs:184,392-403,496-504`
- Agent Builder: `src/actors/agent_builder.rs`
- Tool Macros: `src/tools/macros.rs`

## Related Documentation

- `AGENT_BUILDER_GUIDE.md`: Creating agents with tools
- `MULTI_AGENT_USAGE.md`: Multi-agent patterns
- `HANDOFF_VALIDATION.md`: Quality gates between agents
- `TOOLS_COMPLETE_GUIDE.md`: Creating custom tools
- `REACT_AGENTS.md`: ReAct reasoning pattern
