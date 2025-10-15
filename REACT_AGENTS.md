# ReAct Agents: Multi-Agent Orchestration System

## Table of Contents
1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Configuration](#configuration)
4. [Progress Tracking](#progress-tracking)
5. [Handoff Protocols](#handoff-protocols)
6. [Validation System](#validation-system)
7. [Usage Examples](#usage-examples)
8. [Best Practices](#best-practices)
9. [Troubleshooting](#troubleshooting)

## Overview

The ReAct Agents system implements a multi-agent orchestration framework based on the ReAct (Reasoning and Acting) pattern. This system coordinates multiple specialized agents under a supervisor agent to accomplish complex tasks through iterative reasoning, action, and observation cycles.

### Key Features

- **Multi-Agent Coordination**: Supervisor pattern orchestrating specialized agents
- **Progress Tracking**: Sub-goal based tracking with visual status indicators
- **Handoff Protocols**: Schema-based validation between agent outputs
- **Iteration Budget Management**: Configurable limits at agent and supervisor levels
- **Adaptive Termination**: Auto-completion when all sub-goals are achieved
- **Metadata Enrichment**: Comprehensive execution metrics and tool call tracking
- **Configuration-Driven**: Externalized behavior controls without code changes

### What is ReAct?

ReAct (Reasoning and Acting) is a framework where agents:
1. **Reason**: Think about the task and plan the next action
2. **Act**: Execute tools or make decisions
3. **Observe**: Process results and update understanding
4. **Repeat**: Continue until task completion or iteration limit

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Supervisor Agent                      │
│  - Task decomposition into sub-goals                    │
│  - Progress tracking and completion detection           │
│  - Specialized agent coordination                       │
│  - Budget management (max_orchestration_steps)          │
└───────────────┬─────────────────────────────────────────┘
                │
                ├──────────┬──────────┬──────────┐
                │          │          │          │
           ┌────▼───┐ ┌───▼────┐ ┌──▼─────┐ ┌──▼─────┐
           │ Agent  │ │ Agent  │ │ Agent  │ │ Agent  │
           │   A    │ │   B    │ │   C    │ │   D    │
           └────┬───┘ └───┬────┘ └──┬─────┘ └──┬─────┘
                │         │         │          │
                └─────────┴─────────┴──────────┘
                          │
                    ┌─────▼──────┐
                    │  Handoff   │
                    │ Validation │
                    └────────────┘
```

### Agent Types

#### Supervisor Agent
**Location**: `src/actors/supervisor_agent.rs`

Responsibilities:
- Decompose complex tasks into manageable sub-goals
- Coordinate execution across specialized agents
- Track progress and completion status
- Enforce iteration and sub-goal limits
- Make final decisions on task completion

Key characteristics:
- Has access to ALL specialized agents
- Uses TaskProgress for sub-goal tracking
- Limited by `max_orchestration_steps` config
- Can declare up to `max_sub_goals` upfront

#### Specialized Agents
**Location**: `src/actors/specialized_agent.rs`

Responsibilities:
- Execute domain-specific tasks using ReAct loop
- Use tools to gather information and take actions
- Track execution metrics and tool calls
- Return structured results with metadata

Key characteristics:
- Domain-focused (e.g., database, analysis, web search)
- Limited by `max_iterations` config
- Enrich outputs with execution metadata
- Support handoff validation

### Message Flow

```
User Request
    │
    ▼
┌─────────────────────┐
│   API Layer         │  orchestrate() / orchestrate_with_custom_agents()
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Message Router      │  Route messages to appropriate agent
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Supervisor Agent    │
│                     │  Step 1: Declare sub-goals
│  TaskProgress       │  Step 2-N: Execute sub-goals
│  - Sub-goal 1 [✓]  │  Final Step: Auto-complete
│  - Sub-goal 2 [→]  │
│  - Sub-goal 3 [ ]  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Specialized Agent   │
│                     │  ReAct Loop:
│  Iteration 1/5      │  1. Reason
│  Iteration 2/5      │  2. Act (use tool)
│  ...                │  3. Observe
└──────────┬──────────┘  4. Repeat or finish
           │
           ▼
┌─────────────────────┐
│ Handoff Validation  │  Validate output against contract
└──────────┬──────────┘
           │
           ▼
      Response to User
```

## Configuration

### Configuration File
**Location**: `config/default.toml`

```toml
[llm]
model = "gpt-4"
max_tokens = 2000
temperature = 0.7

[agent]
max_iterations = 5               # Maximum ReAct loop iterations per task
max_orchestration_steps = 5      # Maximum orchestration steps for supervisor
max_sub_goals = 5                # Maximum sub-goals supervisor can declare upfront

[system]
auto_restart = true
heartbeat_interval_ms = 200
heartbeat_timeout_ms = 500
check_interval_ms = 200
channel_buffer_size = 100

[logging]
level = "info"
```

### Configuration Parameters Explained

#### `max_iterations`
**Scope**: Individual specialized agents
**Purpose**: Limits the number of ReAct loop iterations per agent task
**Impact**: Prevents infinite loops in reasoning cycles

Example:
```
Task: "Analyze database performance"
Iteration 1: Reason → Query database → Observe results
Iteration 2: Reason → Calculate metrics → Observe metrics
Iteration 3: Reason → Identify bottleneck → Return answer
```

**When to adjust**:
- Increase for complex analytical tasks requiring multiple reasoning steps
- Decrease for simple lookup tasks
- Typical range: 3-10

#### `max_orchestration_steps`
**Scope**: Supervisor agent
**Purpose**: Limits the number of orchestration steps supervisor can take
**Impact**: Prevents runaway orchestration with too many agent delegations

Example:
```
Step 1: Declare 5 sub-goals
Step 2: Delegate to Database Agent → Success
Step 3: Delegate to Analysis Agent → Success
Step 4: Delegate to Reporting Agent → Success
Step 5: Auto-complete (all sub-goals done)
```

**When to adjust**:
- Increase for complex multi-stage workflows
- Decrease for simple supervised tasks
- Should be ≥ `max_sub_goals` + 1 (for declaration step)
- Typical range: 5-15

#### `max_sub_goals`
**Scope**: Supervisor planning
**Purpose**: Limits how many sub-goals supervisor can declare upfront
**Impact**: Prevents over-planning and forces prioritization

Example:
```
Without limit: LLM might declare 10-20 granular sub-goals
With max_sub_goals=5: LLM intelligently combines steps into 5 high-value goals

User task: "Analyze sales data and generate report"

Without limit (10 sub-goals):
1. Connect to database
2. Validate credentials
3. Query sales table
4. Filter by date range
5. Calculate total revenue
6. Calculate average order value
7. Identify top products
8. Generate charts
9. Format report
10. Save to file

With max_sub_goals=5 (optimized):
1. Connect to database and retrieve sales data
2. Calculate key metrics (revenue, AOV, top products)
3. Generate visualizations
4. Create formatted report
5. Export final deliverable
```

**When to adjust**:
- Increase for genuinely complex workflows with many distinct phases
- Decrease for focused, well-scoped tasks
- LLMs adapt intelligently when constrained
- Typical range: 3-8

### Configuration Interactions

The three limits work together to create a bounded execution environment:

```
max_sub_goals          = 5   (Planning limit)
max_orchestration_steps = 6   (Execution limit: 1 declaration + 5 delegations)
max_iterations         = 5   (Per-agent reasoning limit)

Total worst-case iterations = max_orchestration_steps × max_iterations
                           = 6 × 5 = 30 LLM calls maximum
```

### Environment Variables

```bash
# Required
export OPENAI_API_KEY="your-api-key-here"

# Optional
export CONFIG_ENV="production"  # Loads config/production.toml instead of config/default.toml
export APP__AGENT__MAX_ITERATIONS=10  # Override via environment variable
```

## Progress Tracking

### TaskProgress Structure
**Location**: `src/actors/supervisor_agent.rs:21-80`

The supervisor uses TaskProgress to track sub-goals throughout execution:

```rust
struct TaskProgress {
    sub_goals: Vec<SubGoal>,
    completed_count: usize,
    failed_count: usize,
}

struct SubGoal {
    id: String,
    description: String,
    status: SubGoalStatus,
}

enum SubGoalStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}
```

### Visual Status Indicators

Progress logs use visual markers:
- `[✓]` Complete - Sub-goal successfully finished
- `[→]` In Progress - Currently being executed
- `[ ]` Pending - Not yet started
- `[✗]` Failed - Execution failed

Example output:
```
[SupervisorAgent] Current Progress: 60.00% (3/5 sub-goals completed)
[SupervisorAgent] Sub-goal status:
  [✓] 1. Connect to database and retrieve sales data
  [✓] 2. Calculate key metrics (revenue, AOV, top products)
  [✓] 3. Generate visualizations
  [→] 4. Create formatted report (in progress)
  [ ] 5. Export final deliverable
```

### Sub-Goal Declaration Protocol

The supervisor MUST declare all sub-goals in the first response:

```json
{
  "type": "delegate",
  "agent": "database_agent",
  "task": "Connect to database and retrieve sales data",
  "sub_goals": [
    {
      "id": "1",
      "description": "Connect to database and retrieve sales data"
    },
    {
      "id": "2",
      "description": "Calculate key metrics"
    },
    {
      "id": "3",
      "description": "Generate visualizations"
    },
    {
      "id": "4",
      "description": "Create formatted report"
    },
    {
      "id": "5",
      "description": "Export final deliverable"
    }
  ]
}
```

### Auto-Completion Logic

The supervisor automatically completes when:
1. All declared sub-goals have succeeded
2. Current step resulted in success

```rust
// After each successful agent execution
if task_progress.all_completed() {
    tracing::info!("[SupervisorAgent] All sub-goals completed after this success - finalizing");
    return Ok(AgentResponse::Success {
        result: format!("Task completed successfully. Progress: {}", task_progress.detailed_status()),
        steps,
        metadata: None,
        completion_status: Some(CompletionStatus::Complete { confidence: 1.0 }),
    });
}
```

### Progress Percentage Calculation

```rust
fn progress_percentage(&self) -> f32 {
    if self.sub_goals.is_empty() {
        0.0
    } else {
        self.completed_count as f32 / self.sub_goals.len() as f32
    }
}
```

## Handoff Protocols

### Overview

Handoff protocols ensure quality and consistency when work passes between agents. The system provides schema-based validation with contracts defining expectations.

### Core Components

#### OutputSchema
**Location**: `src/actors/messages.rs:86-95`

Defines the structure and validation rules for agent outputs:

```rust
pub struct OutputSchema {
    pub schema_version: String,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub field_types: HashMap<String, String>,
    pub validation_rules: Vec<ValidationRule>,
}
```

#### ValidationRule
**Location**: `src/actors/messages.rs:97-105`

Supported validation types:

```rust
pub enum ValidationRule {
    MinLength { field: String, min: usize },
    MaxLength { field: String, max: usize },
    Pattern { field: String, regex: String },
    Range { field: String, min: f64, max: f64 },
    Enum { field: String, allowed_values: Vec<String> },
}
```

#### HandoffContract
**Location**: `src/actors/handoff.rs:37-53`

Defines expectations between agents:

```rust
pub struct HandoffContract {
    pub from_agent: String,
    pub to_agent: Option<String>,  // None = final output
    pub schema: OutputSchema,
    pub required_confidence: f32,
    pub max_execution_time_ms: Option<u64>,
}
```

### HandoffCoordinator

**Location**: `src/actors/handoff.rs:55-143`

Central validation coordinator that:
- Registers handoff contracts
- Validates agent outputs against contracts
- Checks confidence thresholds
- Enforces execution time limits
- Provides built-in contract templates

#### Registration

```rust
let mut coordinator = HandoffCoordinator::new();

coordinator.register_contract(
    "database_to_analysis".to_string(),
    HandoffContract {
        from_agent: "database_agent".to_string(),
        to_agent: Some("analysis_agent".to_string()),
        schema: database_schema,
        required_confidence: 0.8,
        max_execution_time_ms: Some(5000),
    }
);
```

#### Validation

```rust
let validation = coordinator.validate_handoff(
    "database_to_analysis",
    &agent_response
)?;

if !validation.valid {
    tracing::error!("Handoff validation failed: {:?}", validation.errors);
    // Handle errors...
}
```

#### Built-in Templates

```rust
// Database output contract
let db_contract = HandoffCoordinator::database_output_contract();
coordinator.register_contract("db_output".to_string(), db_contract);

// Analysis output contract
let analysis_contract = HandoffCoordinator::analysis_output_contract();
coordinator.register_contract("analysis_output".to_string(), analysis_contract);

// API response contract
let api_contract = HandoffCoordinator::api_response_contract();
coordinator.register_contract("api_output".to_string(), api_contract);
```

## Validation System

### OutputValidator
**Location**: `src/actors/validation.rs:10-235`

Core validation engine that checks:
- Required field presence
- Field type correctness
- Validation rule compliance

#### Schema Registration

```rust
let mut validator = OutputValidator::new();

validator.register_schema(
    "user_profile".to_string(),
    OutputSchema {
        schema_version: "1.0".to_string(),
        required_fields: vec!["user_id".to_string(), "email".to_string()],
        optional_fields: vec!["phone".to_string()],
        field_types: {
            let mut types = HashMap::new();
            types.insert("user_id".to_string(), "string".to_string());
            types.insert("email".to_string(), "string".to_string());
            types.insert("phone".to_string(), "string".to_string());
            types
        },
        validation_rules: vec![
            ValidationRule::Pattern {
                field: "email".to_string(),
                regex: r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string(),
            },
            ValidationRule::MinLength {
                field: "user_id".to_string(),
                min: 5,
            },
        ],
    }
);
```

#### Validation Execution

```rust
let output = json!({
    "user_id": "usr_12345",
    "email": "user@example.com",
    "phone": "+1234567890"
});

let result = validator.validate("user_profile", &output);

if result.valid {
    println!("Validation passed!");
} else {
    for error in result.errors {
        eprintln!("Error: {} - {}", error.error_type, error.message);
    }
}
```

### ValidationResult
**Location**: `src/actors/messages.rs:107-112`

```rust
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}
```

### ValidationError
**Location**: `src/actors/messages.rs:114-120`

```rust
pub struct ValidationError {
    pub field: String,
    pub error_type: String,
    pub message: String,
}
```

## Usage Examples

### Example 1: Basic Orchestration

```rust
use llm_fusion::api;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize system
    llm_fusion::init().await?;

    // Run orchestration
    let result = api::orchestrate(
        "Analyze the sales database and generate a comprehensive report"
    ).await?;

    println!("Task completed: {}", result.success);
    println!("Final result: {}", result.final_result);

    llm_fusion::shutdown().await?;
    Ok(())
}
```

### Example 2: Custom Agent Configuration

```rust
use llm_fusion::api;
use llm_fusion::tools::Tool;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    llm_fusion::init().await?;

    // Define custom agents
    let agents = vec![
        (
            "database_specialist".to_string(),
            "Expert in SQL queries and database optimization".to_string(),
            "You are a database specialist...".to_string(),
            vec![database_tool] as Vec<Arc<dyn Tool>>,
        ),
        (
            "data_analyst".to_string(),
            "Expert in statistical analysis and visualization".to_string(),
            "You are a data analyst...".to_string(),
            vec![analysis_tool] as Vec<Arc<dyn Tool>>,
        ),
    ];

    let result = api::orchestrate_with_custom_agents(
        agents,
        "Analyze customer churn and identify key factors"
    ).await?;

    println!("Analysis complete: {:?}", result);

    llm_fusion::shutdown().await?;
    Ok(())
}
```

### Example 3: Handoff Validation

**Full example**: `examples/handoff_validation_example.rs`

```rust
use llm_fusion::actors::handoff::HandoffCoordinator;
use llm_fusion::actors::messages::*;

fn main() {
    let mut coordinator = HandoffCoordinator::new();

    // Use built-in template
    let db_contract = HandoffCoordinator::database_output_contract();
    coordinator.register_contract("db_query".to_string(), db_contract);

    // Create agent response
    let response = AgentResponse::Success {
        result: "Query executed successfully".to_string(),
        steps: vec![],
        metadata: Some(OutputMetadata {
            confidence: 0.95,
            execution_time_ms: 1200,
            agent_name: Some("database_agent".to_string()),
            tool_calls: vec![],
            tokens_used: None,
            partial_results: HashMap::new(),
            schema_version: Some("1.0".to_string()),
            validation_result: None,
        }),
        completion_status: Some(CompletionStatus::Complete { confidence: 0.95 }),
    };

    // Validate handoff
    let validation = coordinator.validate_handoff("db_query", &response);

    match validation {
        Ok(result) => {
            if result.valid {
                println!("Handoff validation passed!");
            } else {
                println!("Validation failed:");
                for error in result.errors {
                    println!("  - {}: {}", error.field, error.message);
                }
            }
        }
        Err(e) => eprintln!("Validation error: {}", e),
    }
}
```

### Example 4: Progress Monitoring

```rust
use llm_fusion::api;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    llm_fusion::init().await?;

    // Start task
    tokio::spawn(async {
        api::orchestrate("Complex multi-step analysis task").await
    });

    // Monitor progress
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let state = llm_fusion::get_system_state().await?;
        println!("Active agents: {}", state.active_agents.len());

        // Check logs for progress updates:
        // [SupervisorAgent] Current Progress: 60.00% (3/5 sub-goals completed)
    }
}
```

## Best Practices

### Configuration Tuning

#### For Simple Tasks
```toml
[agent]
max_iterations = 3
max_orchestration_steps = 3
max_sub_goals = 3
```
Use when: Lookup tasks, simple queries, straightforward operations

#### For Medium Complexity
```toml
[agent]
max_iterations = 5
max_orchestration_steps = 5
max_sub_goals = 5
```
Use when: Multi-step analysis, basic workflows, standard reports

#### For Complex Tasks
```toml
[agent]
max_iterations = 10
max_orchestration_steps = 15
max_sub_goals = 8
```
Use when: Complex analysis, multi-stage pipelines, comprehensive reports

### Sub-Goal Planning

**Good sub-goals** (specific, measurable, achievable):
```
1. Connect to database and retrieve sales data for Q4 2024
2. Calculate revenue, growth rate, and customer metrics
3. Identify top 10 products and bottom 10 performers
4. Generate comparison charts (current vs previous quarter)
5. Create executive summary with key insights
```

**Poor sub-goals** (too granular, not actionable):
```
1. Think about the problem
2. Consider what data we need
3. Maybe query the database
4. Look at the results
5. Do some calculations
6. Make a chart
7. Write something
8. Check if it looks good
9. Finalize
10. Done
```

### Agent Design

**Specialized agents should**:
- Focus on a specific domain (database, analysis, web search)
- Have clear tool sets relevant to their domain
- Return structured outputs matching expected schemas
- Include execution metadata for observability

**Specialized agents should NOT**:
- Try to do everything (leave orchestration to supervisor)
- Make decisions about overall task flow
- Communicate directly with other agents (use supervisor)

### Validation Strategy

**Always validate when**:
- Output will be consumed by another agent
- Output format is critical for downstream processing
- Confidence thresholds matter for decision-making

**Validation is optional when**:
- Final output to user (user can judge quality)
- Internal logging or debugging
- Prototype or experimental features

### Error Handling

**Graceful degradation**:
```rust
match agent_result {
    AgentResponse::Success { .. } => {
        // Happy path
    }
    AgentResponse::Failure { error, completion_status, .. } => {
        if let Some(CompletionStatus::Failed { recoverable: true, .. }) = completion_status {
            // Retry with different approach
        } else {
            // Fail fast and report to user
        }
    }
    AgentResponse::Timeout { .. } => {
        // Consider increasing max_iterations or simplifying task
    }
}
```

## Troubleshooting

### Issue: Agent times out without completing task

**Symptoms**:
```
[SpecializedAgent] Reached maximum iterations (5) without finding final answer
```

**Diagnosis**:
- Task is too complex for current `max_iterations` setting
- Agent is stuck in reasoning loop without progress
- Tools are returning insufficient information

**Solutions**:
1. Increase `max_iterations` in config
2. Simplify the task or break into smaller sub-goals
3. Review agent's system prompt for clarity
4. Check tool outputs for usefulness

### Issue: Supervisor declares too many sub-goals

**Symptoms**:
```
[SupervisorAgent] LLM declared 10 sub-goals, but max_sub_goals is 5. Truncating to first 5.
```

**Diagnosis**:
- Task genuinely requires many steps
- LLM is being too granular in planning
- `max_sub_goals` is set too low for task complexity

**Solutions**:
1. Increase `max_sub_goals` if task warrants it
2. Trust LLM to intelligently combine steps when constrained
3. Review supervisor prompt for over-planning tendencies
4. Consider if task scope is too broad

### Issue: Handoff validation fails

**Symptoms**:
```
Handoff validation failed: [ValidationError { field: "data", error_type: "missing_required_field" }]
```

**Diagnosis**:
- Agent output doesn't match expected schema
- Schema is too strict for actual use case
- Agent is not aware of output requirements

**Solutions**:
1. Update agent prompt to specify required output format
2. Adjust schema to match actual output structure
3. Add schema information to agent's context
4. Check if optional fields should be required

### Issue: Progress shows 100% but task not complete

**Symptoms**:
```
[SupervisorAgent] Current Progress: 100.00% (5/5 sub-goals completed)
[SupervisorAgent] Continuing orchestration (auto-completion not triggered)
```

**Diagnosis**:
- Auto-completion logic not detecting completion state
- Sub-goals marked complete prematurely
- Additional steps generated after initial planning

**Solutions**:
1. Review sub-goal completion criteria in logs
2. Ensure all sub-goals are declared in first step
3. Check for edge case in auto-completion logic
4. Verify supervisor is not creating new sub-goals mid-execution

### Issue: Execution is too slow

**Symptoms**:
- Each iteration takes multiple seconds
- Total execution time exceeds expectations

**Diagnosis**:
- LLM API latency
- Too many iterations/steps configured
- Inefficient tool implementations

**Solutions**:
1. Reduce `max_iterations` and `max_orchestration_steps`
2. Optimize tool implementations for speed
3. Use faster LLM model with lower latency
4. Consider caching for repeated queries
5. Profile tool execution times in metadata

### Issue: Inconsistent results between runs

**Symptoms**:
- Same task produces different sub-goal counts
- Completion status varies unpredictably

**Diagnosis**:
- LLM temperature too high (non-deterministic)
- Insufficient prompt clarity
- Edge cases in validation logic

**Solutions**:
1. Lower `temperature` in config (e.g., 0.3 for more deterministic)
2. Make system prompts more explicit and directive
3. Add examples to prompts for consistency
4. Review validation logic for edge cases

### Debugging Tips

**Enable detailed logging**:
```toml
[logging]
level = "debug"  # or "trace" for even more detail
```

**Check system state**:
```rust
let state = llm_fusion::get_system_state().await?;
println!("Active agents: {:?}", state.active_agents);
println!("Last heartbeats: {:?}", state.last_heartbeats);
```

**Inspect agent metadata**:
```rust
if let AgentResponse::Success { metadata: Some(meta), .. } = response {
    println!("Execution time: {}ms", meta.execution_time_ms);
    println!("Tool calls: {}", meta.tool_calls.len());
    println!("Confidence: {}", meta.confidence);
}
```

**Monitor progress logs**:
Look for patterns in sub-goal status transitions:
```
[✓] → [→] → [✓]  (healthy progression)
[✓] → [✗] → [✓]  (retry after failure)
[ ] → [ ] → [ ]  (stuck, not progressing)
```

## Advanced Topics

### Custom Validation Rules

Extend ValidationRule enum for domain-specific checks:

```rust
// In src/actors/messages.rs
pub enum ValidationRule {
    // Existing rules...
    MinLength { field: String, min: usize },
    MaxLength { field: String, max: usize },

    // Add custom rules:
    Custom {
        field: String,
        validator: fn(&Value) -> bool,
        error_message: String
    },
}
```

### Metadata Enrichment Pipeline

```rust
// In src/actors/specialized_agent.rs
fn enrich_metadata(
    base_metadata: OutputMetadata,
    validation: ValidationResult,
    custom_data: HashMap<String, String>,
) -> OutputMetadata {
    OutputMetadata {
        validation_result: Some(validation),
        partial_results: custom_data,
        ..base_metadata
    }
}
```

### Dynamic Configuration

```rust
// Override config at runtime
let mut settings = Settings::new()?;
settings.agent.max_iterations = 10;  // Increase for this specific run

let system = System::new(settings, api_key);
```

### Agent Composition Patterns

**Sequential Pipeline**:
```
DataExtractor → Transformer → Analyzer → Reporter
```

**Parallel Execution**:
```
              ┌→ Analyzer A ─┐
DataExtractor ├→ Analyzer B ─┤→ Aggregator → Reporter
              └→ Analyzer C ─┘
```

**Hierarchical Supervision**:
```
Master Supervisor
    ├→ Data Pipeline Supervisor
    │       ├→ Extractor
    │       └→ Transformer
    └→ Analysis Supervisor
            ├→ Statistical Analyzer
            └→ ML Analyzer
```

## Performance Considerations

### Token Usage

Each iteration consumes tokens:
- System prompt: ~500-1000 tokens
- User task: ~50-200 tokens
- Reasoning: ~200-500 tokens
- Tool results: ~100-1000 tokens per tool

**Optimization strategies**:
1. Minimize system prompt length while maintaining clarity
2. Use smaller models for simple tasks
3. Cache repeated queries
4. Limit tool output verbosity

### Execution Time

Typical latencies:
- LLM API call: 1-5 seconds
- Tool execution: 0.1-2 seconds
- Validation: <0.01 seconds

**Total time estimate**:
```
Total = (max_orchestration_steps × (LLM_latency + tool_latency))
      + (max_sub_goals × max_iterations × (LLM_latency + tool_latency))

Example with config (5, 5, 5):
Total ≈ (5 × 3s) + (5 × 5 × 3s) = 15s + 75s = 90s worst case
```

### Memory Usage

- Each agent: ~10-50 MB (depending on tool data)
- Message history: ~1 KB per step
- Validation schemas: ~10 KB total

**For large-scale deployments**:
- Limit concurrent orchestrations
- Implement message history pruning
- Stream large tool outputs

## References

### Code Locations

- **Configuration**: `config/default.toml`, `src/config/settings.rs`
- **Messages**: `src/actors/messages.rs`
- **Validation**: `src/actors/validation.rs`
- **Handoff**: `src/actors/handoff.rs`
- **Supervisor**: `src/actors/supervisor_agent.rs`
- **Specialized Agents**: `src/actors/specialized_agent.rs`
- **API**: `src/api.rs`
- **Examples**: `examples/handoff_validation_example.rs`

### External Resources

- ReAct Paper: "ReAct: Synergizing Reasoning and Acting in Language Models"
- Multi-Agent Systems: Research on hierarchical agent coordination
- LLM Agent Patterns: Supervisor, router, and specialist patterns

### Version History

- **v1.0**: Initial ReAct agent implementation
- **v1.1**: Added progress tracking with sub-goal declarations
- **v1.2**: Implemented handoff protocols and validation system
- **v1.3**: Externalized all configuration parameters (current)

## Contributing

When extending the system:

1. **Follow information hiding principles**: Encapsulate design decisions
2. **Maintain backward compatibility**: Don't break existing contracts
3. **Add validation schemas**: For new agent types
4. **Update configuration**: Document new parameters in default.toml
5. **Write examples**: Demonstrate new features
6. **Test thoroughly**: Ensure iteration limits work correctly

## License

See LICENSE file in project root.
 Great question! No, the HandoffCoordinator is NOT an agent - it's a validation utility that works alongside agents.

  Let me clarify the distinction:

  HandoffCoordinator vs Agents

  What Agents Are:

  - Autonomous actors that use LLMs to reason and make decisions
  - Execute the ReAct loop (Reason → Act → Observe)
  - Use tools to perform actions
  - Examples: SupervisorAgent, SpecializedAgent

  What HandoffCoordinator Is:

  - A validation library/utility
  - Does NOT use LLMs
  - Does NOT have tools
  - Just checks data against schemas
  - Pure Rust code (no AI involved)

  Think of it like this:

  ┌─────────────────────────────────────────────┐
  │           Agent (uses LLM + tools)          │
  │                                             │
  │  1. Reason: "I need to query database"     │
  │  2. Act: Use DatabaseTool                  │
  │  3. Observe: Got results                   │
  │  4. Return: AgentResponse                  │
  └──────────────────┬──────────────────────────┘
                     │
                     │ AgentResponse
                     ▼
           ┌─────────────────────┐
           │ HandoffCoordinator  │  ← NOT an agent
           │   (Just validator)  │  ← No LLM
           │                     │  ← No tools
           │  ✓ Check schema     │  ← Just checks data
           │  ✓ Check confidence │
           │  ✓ Check timing     │
           └──────────┬──────────┘
                      │
                      │ ValidationResult
                      ▼
           ┌─────────────────────┐
           │   Next Agent        │
           │  (receives valid    │
           │   data to work on)  │
           └─────────────────────┘

  Why No Tools/Macros?

● Read(src/actors/handoff.rs)
  ⎿  Read 100 lines

● Look at the code - it's just pure validation logic:

  pub fn validate_handoff(
      &self,
      contract_name: &str,
      response: &AgentResponse,
  ) -> ValidationResult {
      // Just checking data:
      // 1. Does response have required fields?
      // 2. Is confidence high enough?
      // 3. Did it finish within time limit?
      // 4. Does the data match the schema?

      // No LLM calls
      // No tool execution
      // Just data checking
  }

  Why This Design?

  Tools are for agents that need to DO things:
  - Query database → DatabaseTool
  - Search web → WebSearchTool
  - Calculate → CalculatorTool
  - File operations → FileReadTool

  HandoffCoordinator just VALIDATES data - no actions needed:
  - Check if field exists → Just Rust code
  - Check confidence threshold → Simple comparison
  - Validate regex pattern → Regex library
  - Check execution time → Math comparison

  In Real Usage

  Here's how they work together:

  // ✅ AGENTS use tools (with macros)
  #[tool_fn]
  async fn query_database(query: String) -> ToolResult {
      // Execute SQL query
      Ok(results)
  }

  let db_agent = SpecializedAgent::new(
      "database_agent",
      tools: vec![query_database_tool()],  // ← Agent uses tools
      ...
  );

  // Agent executes and returns result
  let response = db_agent.execute_task("Get sales data", 5).await;

  // ✅ HANDOFF COORDINATOR validates (NO tools, NO macros)
  let coordinator = HandoffCoordinator::new();
  coordinator.register_contract("db_to_analysis", contract);

  let validation = coordinator.validate_handoff("db_to_analysis", &response);
  // ↑ Just checking the response data, no LLM, no tools

  if validation.valid {
      // Pass to next agent
      analysis_agent.execute_task(&response.result, 5).await;
  }

  Summary

  | Feature      | Agent (SupervisorAgent, SpecializedAgent) | HandoffCoordinator |
  |--------------|-------------------------------------------|--------------------|
  | Uses LLM?    | ✅ Yes                                     | ❌ No               |
  | Has tools?   | ✅ Yes (#[tool_fn])                        | ❌ No               |
  | Uses macros? | ✅ Yes (#[tool_fn])                        | ❌ No               |
  | ReAct loop?  | ✅ Yes                                     | ❌ No               |
  | Purpose      | Execute tasks                             | Validate data      |
  | Type         | Autonomous actor                          | Utility library    |
  | Analogy      | Worker/Manager                            | Quality Inspector  |

  Think of it like a factory:
  - Agents = Workers who build things (use tools, make decisions)
  - HandoffCoordinator = Quality inspector with a checklist (just checks if output meets standards)

  The quality inspector doesn't need tools or AI - they just have a checklist and verify the product meets specifications before it goes to the next worker.

  Does this clarify why HandoffCoordinator doesn't use tools/macros?

─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
>
 Let me explain the 6-tuple while I continue fixing the code:

  The 6-tuple is needed because we added the return_tool_output boolean flag to the agent configuration. Here's the breakdown:

  What the tuple contains:
  1. String - Agent name
  2. String - Agent description
  3. String - System prompt
  4. Vec<Arc<dyn Tool>> - Tools the agent can use
  5. Option<serde_json::Value> - Response schema (for structured outputs - which we're removing)
  6. bool - return_tool_output flag (this is the new addition)

  Why we need it:
  The return_tool_output flag tells the agent: "Don't wrap the tool output with explanations, just return the tool's JSON output directly."

  In your database example, tools like QueryProductRevenueTool return perfect JSON, but without this flag, agents would wrap it like:
  "Successfully retrieved product revenue data. The total revenue for each product is..."

  With return_tool_output=true, the agent returns the tool's raw JSON output directly, which is what your validation system expects.

  The feature was already implemented in specialized_agent.rs, but the AgentBuilder.build() method wasn't passing the flag through to the config - that's what we're fixing
  now.

  Let me continue updating the supervisor functions:

Perfect! Now I understand. Let me explain what you have:

  Two APIs

  1. orchestrate_custom_agents() - No Validation
  supervisor::orchestrate_custom_agents(agent_configs, task).await?;
  - Just orchestrates agents
  - No quality gates
  - Trusts agents to return valid data

  2. orchestrate_custom_agents_with_validation() - With Validation
  let coordinator = HandoffCoordinator::new();
  // Register contracts...

  supervisor::orchestrate_custom_agents_with_validation(
      coordinator,
      agent_configs,
      task
  ).await?;
  - Orchestrates agents AND validates outputs
  - Quality gates between agents
  - Checks contracts before passing data

  What HandoffCoordinator Does

  The HandoffCoordinator validates agent outputs against contracts:

  // 1. Define what you expect
  coordinator.register_contract(
      "database_agent_handoff",
      HandoffContract {
          from_agent: "database_agent",
          to_agent: Some("analysis_agent"),
          schema: OutputSchema {
              required_fields: vec!["data", "status"],
              field_types: {"data": "array", "status": "string"},
              validation_rules: vec![
                  // status must be one of these values
                  ValidationRule {
                      field: "status",
                      rule_type: ValidationType::Enum,
                      constraint: "success,partial,failed"
                  },
                  // row_count must be 1-1000
                  ValidationRule {
                      field: "row_count",
                      rule_type: ValidationType::Range,
                      constraint: "1..1000"
                  }
              ]
          },
          max_execution_time_ms: Some(10000), // Must complete in 10s
      }
  );

  // 2. Supervisor validates automatically
  // After database_agent completes:
  let validation = coordinator.validate_handoff("database_agent_handoff", &agent_response);
  if !validation.valid {
      // ❌ Block handoff, retry or fail
  } else {
      // ✅ Pass to analysis_agent
  }

  When Each Is Useful

  Use orchestrate_custom_agents() when:
  - Your tools are type-safe Rust (guaranteed valid output)
  - Using return_tool_output(true) (bypasses LLM hallucination)
  - You trust your pipeline
  - You want less boilerplate

  Use orchestrate_custom_agents_with_validation() when:
  - LLM generates the output (can hallucinate)
  - External APIs that might fail
  - Mission-critical pipeline where bad data is expensive
  - You need audit trail of what passed/failed validation

  Bottom Line

  As we discussed earlier - with your type-safe tools + return_tool_output(true), validation might be overkill. But the infrastructure is there for when you need it (external APIs,
   LLM-generated data, etc.).

  The two APIs give you the choice: simple (no validation) or paranoid (with validation).

 Exactly! That's the beauty of your #[tool_fn] macro design. Let me show you how it all works together:

● Read(examples/supervisor_database_pipeline.rs)
  ⎿  Read 45 lines

● YES! That's the magic. Look at how seamlessly it works:

  Your Macro in Action

  // 1. Write simple async function with SQL
  #[tool_fn(
      name = "query_product_revenue",
      description = "Query total revenue for each product from the sales database"
  )]
  async fn query_product_revenue() -> Result<String> {
      let conn = DB.lock().unwrap();

      // Real SQL!
      let mut stmt = conn.prepare(
          "SELECT product_name, SUM(quantity * unit_price) as total_revenue
           FROM sales GROUP BY product_name"
      )?;

      // Real rusqlite API!
      let rows = stmt.query_map([], |row| {
          Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
      })?;

      // Format results
      Ok(format!("Product Revenue: ..."))
  }

  // 2. Macro auto-generates QueryProductRevenueTool struct
  // 3. Macro implements Tool trait with execute(), validate(), metadata()
  // 4. Agent can now call this tool via LLM

  // 4. Use it in agent
  let database_agent = AgentBuilder::new("database_agent")
      .tool(QueryProductRevenueTool::new())  // ← Auto-generated by macro!
      .build();

  The Full Stack Working Together

  User Task: "Analyze sales data"
           ↓
      Supervisor LLM decides: "Call database_agent"
           ↓
      Database Agent LLM decides: "Use query_product_revenue tool"
           ↓
      #[tool_fn] macro wrapper calls your function
           ↓
      Your function executes real SQL on SQLite
           ↓
      rusqlite returns query results
           ↓
      Your function formats as String
           ↓
      Macro wraps in ToolResult
           ↓
      Agent receives result
           ↓
      Agent returns to Supervisor
           ↓
      Supervisor continues orchestration

  What Makes This Beautiful

  1. Simple function - You write plain async Rust
  2. Real SQL - Full rusqlite power (joins, aggregates, subqueries)
  3. Zero boilerplate - Macro generates all Tool infrastructure
  4. Type-safe - Rust compiler checks SQL types
  5. Composable - Tools can call other tools via parameters

  Example: Tool with Parameters

  #[tool_fn(
      name = "get_top_products",
      description = "Get the top N products by revenue"
  )]
  async fn get_top_products(limit: i64) -> Result<String> {
      let conn = DB.lock().unwrap();

      let mut stmt = conn.prepare(
          "SELECT product_name, SUM(quantity * unit_price)
           FROM sales
           GROUP BY product_name
           LIMIT ?1"  // ← SQL parameter
      )?;

      let rows = stmt.query_map([limit], ...)?;  // ← Pass limit
      Ok(...)
  }

  The macro auto-generates:
  - Parameter extraction: limit: i64
  - Type checking: ensures i64 is provided
  - JSON conversion: LLM passes {"limit": 3}
  - Function call: get_top_products(3).await

  You literally just write the SQL function, slap #[tool_fn] on it, and boom - it's an AI tool!



antirez 9 months ago | next [–]

Lua is great to embed, but for Redis I picked it as a scripting language with many regrets because I don't like the language itself: it looks almost like designed to have some kind of friction compared to what you expect to write in a language that is at the same level of abstractions. Little design decisions here and there that cumulate into a language that is somewhat hostile (for me, at least). Yet, it is so fast, easy to integrate, small footprint and reliable, that many times it is still the way to go. Btw, the hostility is present even at C-API level, with the stack approach: it's not rocket science but more cognitive load than needed. Even if I was exposed in the past to stack languages like FORTH, I still had to do some mental gym while writing the bindings. Why?


ufo 9 months ago | parent | next [–]

Have you seen the Passing a Language Through the Eye of a Needle article, from the Lua authors? It talks about how some of unusual the design decisions such as pcall error handling and the stack are tradeoffs in favor of "embeddability".
https://queue.acm.org/detail.cfm?id=1983083

The main purpose of the stack is to facilitate garbage collection. The GC can find out what Lua objects are currently being manipulated by the C client. The price is that the Lua API can never expose a "Lua object pointer" to C. Many other scripting languages expose such pointers, but then must also expose ways to manage their memory. For example, in Python the user of the API must explicitly increment and decrement reference counts.



ranger_danger 9 months ago | root | parent | next [–]

The one time I tried to embed Lua in a C project (which included C-based functions I could call from Lua and setting up variables from C that were visible in Lua), I constantly struggled with trying to figure out when I needed to push/pop things from the stack and it just seemed very error-prone and easy to leak memory with. Different functions seem to have different requirements for when/if you should push/pop things and the documentation was not always clear to me.
Has this changed any?



ufo 9 months ago | root | parent | next [–]

The stack's still there. I agree that reference manual's notation for how many values are pushed and popped can take a while to get used to, but at least it's straight to the point.
One trap with the stack is that it baits you into carefully sequencing the operations so that the output of one feeds into the input the the next. Sometimes values are popped far from the places that pushed them... It can be easier to reason about code that liberally copies the temporary values. Keep one stack slot for each "local variable" you want to work with. Then to work on them you copy that slot to the top, call the stack operation, and then write the result back to the appropriate stack slot.

Essentially, favor positive stack indices over negative indices, because the latter are more sensitive to the sequencing of your operations. Also, consider giving names to the stack indices instead of hardcoded numbers.



matheusmoreira 9 months ago | parent | prev | next [–]

> the hostility is present even at C-API level, with the stack approach
> Why?

I suspect it's because implementing control mechanisms is much easier when you reify the language's stack. Especially advanced ones such as generators and continuations which require copying the stack into an object and restoring it later. Making that work with the native stack is really hard and many languages don't even try.

It also makes garbage collection precise. Lua values in C variables would be placed in registers or the native stack. In order to trace those values, Lua would require a conservative garbage collector that spills the registers and scans the entire native stack. By managing their own stack, they can avoid doing all that.



astrobe_ 9 months ago | parent | prev | next [–]

A historical thing:
> Traditionally, most virtual machines intended for actual execution are stack based, a trend that started with Pascal’s P-machine and continues today with Java’s JVM and Microsoft’s .Net environment. Currently, however, there has been a growing interest in register-based virtual machines (for instance, the planned new virtual machine for Perl 6 (Parrot) will be register based). As far as we know, the virtual machine of Lua 5.0 is the first register-based virtual machine to have a wide use.

The API directly reflected the (previous) internals of the VM, I guess [1].

[1] (pdf) https://www.lua.org/doc/jucs05.pdf



ufo 9 months ago | root | parent | next [–]

The new register-based virtual machine still uses the same stack under the hood. The only difference is how instructions are encoded in the bytecode. By default they can read and write to any position in the stack as opposed to pushing and popping from the top. The Lua stack API already does some of this, because several operations take stack indices as arguments.
