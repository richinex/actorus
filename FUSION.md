# LLM Fusion Architecture - Memory Sharing and Orchestration

## Overview

LLM Fusion uses the **Actor Model** (inspired by Erlang) for reliable, scalable multi-agent coordination. This document explains how memory is shared (spoiler: it isn't) and how the supervisor orchestrates multiple specialized agents.

## Memory Sharing Architecture

### No Shared Memory - Message Passing Only

```
┌──────────────┐      ┌──────────────┐      ┌──────────────┐
│ Supervisor   │      │ Data Agent   │      │ Math Agent   │
│   Agent      │      │              │      │              │
└──────┬───────┘      └──────┬───────┘      └──────┬───────┘
       │                     │                     │
       │   Messages Only     │                     │
       └─────────────────────┴─────────────────────┘
                   No Shared State
```

**Key Principles:**
- Each agent runs in its own async task
- Agents communicate ONLY through messages (tokio channels)
- No shared mutable state between agents
- Each agent has its own tools and LLM client

### Component Isolation

From `src/actors/supervisor_agent.rs:84-152`:

```rust
pub struct SupervisorAgent {
    agents: Vec<SpecializedAgent>,  // Each agent is independent
    llm_client: LLMClient,           // Supervisor has its own LLM client
}
```

Each `SpecializedAgent` is created with:
- **Own LLMClient**: Independent connection to LLM API
- **Own ToolRegistry**: Isolated tool collection
- **Own ToolExecutor**: Independent execution engine
- **Own Memory**: Separate conversation history

## Supervisor Orchestration Process

### Complete Flow Breakdown

#### Step 1: Task Arrives at Supervisor

From `src/actors/supervisor_agent.rs:156-160`:

```rust
pub async fn orchestrate(&self, task: &str, max_steps: usize) -> AgentResponse {
    tracing::info!("[SupervisorAgent] Orchestrating task: {}", task);

    let mut orchestration_history = Vec::new();
    let mut step_count = 0;
```

**Example Task:**
```
"Add 100 units of 'Premium Widget' to inventory in Electronics category,
 then count the total inventory, and finally calculate what 25% of that total would be"
```

#### Step 2: Supervisor Constructs Planning Prompt

From `src/actors/supervisor_agent.rs:162-187`:

```rust
let system_prompt = format!(
    "You are a supervisor coordinating multiple specialized agents.\n\n\
     Available agents:\n{}\n\n\
     Your job is to:\n\
     1. Break down complex tasks into smaller subtasks\n\
     2. Decide which agent should handle each subtask\n\
     3. Invoke agents one at a time\n\
     4. Track progress and determine when task is complete\n\n\
     Respond in JSON format:\n\
     {{\n\
       \"thought\": \"your reasoning\",\n\
       \"agent_name\": \"name_of_agent\",\n\
       \"agent_task\": \"specific task for agent\",\n\
       \"is_complete\": false\n\
     }}",
    self.agents_description()
);
```

**The Supervisor's Context Includes:**
- Available agents and their capabilities:
  - `data_agent`: Manages inventory data
  - `text_agent`: Processes text
  - `math_agent`: Performs calculations
- Previous orchestration steps (if any)
- The current task to accomplish

#### Step 3: LLM Plans First Action

The supervisor sends to its LLM:
- System prompt (above)
- Task description
- Previous actions (empty on first iteration)

**LLM Response (JSON):**
```json
{
  "thought": "I need to first add the items to inventory, then count total, then calculate percentage",
  "agent_name": "data_agent",
  "agent_task": "add 100 units of 'Premium Widget' to inventory in Electronics category",
  "is_complete": false
}
```

#### Step 4: Supervisor Invokes Chosen Agent

From `src/actors/supervisor_agent.rs:224-254`:

```rust
// Find the agent
let chosen_agent = self.agents.iter()
    .find(|a| a.name() == decision.agent_name)
    .ok_or_else(|| anyhow::anyhow!("Agent '{}' not found", decision.agent_name))?;

tracing::info!(
    "[SupervisorAgent] Invoking '{}' with task: {}",
    decision.agent_name,
    decision.agent_task
);

// Execute the agent's task (blocking call - waits for result)
let agent_response = chosen_agent.execute_task(
    &decision.agent_task,
    10  // max iterations for this agent's ReAct loop
).await;
```

**Key Point:** The supervisor calls the agent synchronously and waits for the result. No parallelism here - it's sequential delegation.

#### Step 5: Agent Executes Task Independently

From `src/actors/specialized_agent.rs:84-290`:

Each agent has its **own ReAct loop** that runs completely independently:

```rust
pub async fn execute_task(&self, task: &str, max_iterations: usize) -> AgentResponse {
    let mut steps = Vec::new();
    let mut conversation_history = Vec::new();

    // Build system prompt with agent's available tools
    let system_prompt = format!(
        "{}\n\nAvailable Tools:\n{}\n\n\
         IMPORTANT: You MUST respond in this EXACT JSON format:\n\
         {{\n  \
           \"thought\": \"your reasoning about what to do next\",\n  \
           \"action\": {{\"tool\": \"tool_name\", \"input\": {{\"param\": \"value\"}}}},\n  \
           \"is_final\": false,\n  \
           \"final_answer\": null\n\
         }}",
        self.config.system_prompt,
        self.tool_registry.tools_description()
    );

    conversation_history.push(ChatMessage {
        role: "system".to_string(),
        content: system_prompt,
    });

    conversation_history.push(ChatMessage {
        role: "user".to_string(),
        content: format!("Task: {}", task),
    });

    // ReAct loop: Think → Act → Observe
    for iteration in 0..max_iterations {
        // THINK: Agent asks its own LLM what to do
        let decision = match self.think(&conversation_history).await {
            Ok(d) => d,
            Err(e) => {
                return AgentResponse::Failure {
                    error: format!("Failed to reason: {}", e),
                    steps,
                };
            }
        };

        // Check if task is complete
        if decision.is_final {
            let final_answer = decision.final_answer.unwrap_or_else(|| {
                "Task completed".to_string()
            });

            steps.push(AgentStep {
                iteration,
                thought: decision.thought.clone(),
                action: None,
                observation: Some(final_answer.clone()),
            });

            return AgentResponse::Success {
                result: final_answer,
                steps,
            };
        }

        // ACT: Execute the tool
        if let Some(action) = decision.action {
            let tool = match self.tool_registry.get(&action.tool) {
                Some(t) => t,
                None => {
                    let error_msg = format!("Tool '{}' not found", action.tool);
                    conversation_history.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: format!("Error: {}", error_msg),
                    });

                    steps.push(AgentStep {
                        iteration,
                        thought: decision.thought,
                        action: Some(action.tool.clone()),
                        observation: Some(error_msg),
                    });
                    continue;
                }
            };

            // Execute tool
            let tool_result = match self.tool_executor.execute(tool, action.input.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    let error_msg = format!("Tool execution failed: {}", e);
                    conversation_history.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: error_msg.clone(),
                    });

                    steps.push(AgentStep {
                        iteration,
                        thought: decision.thought,
                        action: Some(action.tool.clone()),
                        observation: Some(error_msg),
                    });
                    continue;
                }
            };

            // OBSERVE: Agent sees the result
            let observation = if tool_result.success {
                tool_result.output.clone()
            } else {
                format!("Tool failed: {}", tool_result.error.unwrap_or_default())
            };

            // Add action and observation to conversation
            conversation_history.push(ChatMessage {
                role: "assistant".to_string(),
                content: serde_json::to_string(&AgentDecision {
                    thought: decision.thought.clone(),
                    action: Some(action.clone()),
                    is_final: false,
                    final_answer: None,
                }).unwrap_or_else(|_| format!("Action: {}", action.tool)),
            });

            conversation_history.push(ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Observation: {}\n\nDoes this observation contain the answer? \
                     If yes, set is_final=true. If no, what is the next action?",
                    observation
                ),
            });

            steps.push(AgentStep {
                iteration,
                thought: decision.thought,
                action: Some(action.tool.clone()),
                observation: Some(observation),
            });
        }
    }

    // Max iterations reached
    AgentResponse::Timeout {
        partial_result: "Max iterations reached without completing task".to_string(),
        steps,
    }
}
```

**Critical Insight:** The agent doesn't know it's being orchestrated. From its perspective:
1. It receives a task string
2. It uses its tools to complete the task
3. It returns a result
4. **It forgets everything** (no persistent state)

#### Step 6: Supervisor Receives Result

From `src/actors/supervisor_agent.rs:256-271`:

```rust
match agent_response {
    AgentResponse::Success { result, steps } => {
        tracing::info!(
            "[SupervisorAgent] Agent '{}' result: SUCCESS: {}",
            decision.agent_name,
            result
        );

        // Add to orchestration history
        orchestration_history.push(SupervisorStep {
            thought: decision.thought.clone(),
            agent_invoked: Some(decision.agent_name.clone()),
            agent_task: Some(decision.agent_task.clone()),
            agent_result: Some(format!("SUCCESS: {}", result)),
        });

        // Add result to conversation for next planning step
        conversation_history.push(ChatMessage {
            role: "assistant".to_string(),
            content: serde_json::to_string(&decision).unwrap(),
        });

        conversation_history.push(ChatMessage {
            role: "user".to_string(),
            content: format!(
                "Agent '{}' completed subtask successfully.\n\
                 Result: {}\n\n\
                 What should I do next? Is the overall task complete?",
                decision.agent_name, result
            ),
        });
    }
    AgentResponse::Failure { error, steps } => {
        // Handle failure (add to history, potentially retry or fail)
    }
    AgentResponse::Timeout { partial_result, steps } => {
        // Handle timeout
    }
}
```

#### Step 7: Supervisor Plans Next Action

The supervisor loops back to Step 2, but now with enriched context:

**Updated Conversation History:**
```
System: "You are a supervisor..."
User: "Task: Add 100 units... then count... then calculate 25%"
Assistant: {"thought": "Add items first", "agent_name": "data_agent", ...}
User: "Agent 'data_agent' completed. Result: Added 100 units. What next?"
```

**The supervisor asks its LLM again:** "What should I do next?"

**LLM Response:**
```json
{
  "thought": "Items added successfully. Now I need to count total inventory.",
  "agent_name": "data_agent",
  "agent_task": "count total inventory",
  "is_complete": false
}
```

#### Step 8: Repeat Until Complete

This process continues:

**Iteration 2:**
- Invoke: `data_agent` with "count total inventory"
- Result: "Total items in inventory: 15"

**Iteration 3:**
- Invoke: `math_agent` with "calculate 25% of 15"
- Result: "25% of 15 = 3"

**Iteration 4:**
- LLM responds with `"is_complete": true`
- Supervisor returns final combined result

## Memory and State Management

### What Each Component Remembers

**Supervisor State:**
```rust
struct SupervisorAgent {
    agents: Vec<SpecializedAgent>,           // Static agent pool
    llm_client: LLMClient,                   // For planning
}

// During orchestration:
let mut orchestration_history: Vec<SupervisorStep> = vec![
    SupervisorStep {
        thought: "Add items first",
        agent_invoked: Some("data_agent"),
        agent_task: Some("add 100 units..."),
        agent_result: Some("SUCCESS: Added 100 units"),
    },
    SupervisorStep {
        thought: "Now count inventory",
        agent_invoked: Some("data_agent"),
        agent_task: Some("count total inventory"),
        agent_result: Some("SUCCESS: Total: 15"),
    },
    // ... more steps
];

let mut conversation_history: Vec<ChatMessage> = vec![
    ChatMessage { role: "system", content: "You are a supervisor..." },
    ChatMessage { role: "user", content: "Task: ..." },
    ChatMessage { role: "assistant", content: "{...decision...}" },
    ChatMessage { role: "user", content: "Result: ..." },
    // ... more messages
];
```

**Agent State (During Execution):**
```rust
// Inside execute_task()
let mut conversation_history: Vec<ChatMessage> = vec![
    ChatMessage { role: "system", content: "You have tools: add_item, search_items..." },
    ChatMessage { role: "user", content: "Task: add 100 units..." },
    ChatMessage { role: "assistant", content: "{...thinking...}" },
    ChatMessage { role: "user", content: "Observation: Added successfully" },
    // ... ReAct loop messages
];

let mut steps: Vec<AgentStep> = vec![
    AgentStep {
        iteration: 0,
        thought: "I need to use add_item tool",
        action: Some("add_item"),
        observation: Some("Added 100 units of 'Premium Widget'..."),
    },
    // ... more steps
];
```

**After Agent Returns:**
- `conversation_history` is discarded
- `steps` are returned to supervisor
- Agent has no memory of this execution

### Important: Agents Are Stateless

When the supervisor calls `data_agent` twice:

**First Call:**
```
data_agent.execute_task("add 100 units...")
→ Uses tools, converses with LLM, completes task
→ Returns: "Added 100 units"
→ FORGETS EVERYTHING
```

**Second Call:**
```
data_agent.execute_task("count total inventory")
→ Starts fresh, no memory of previous call
→ Uses tools, converses with LLM
→ Returns: "Total: 15"
→ FORGETS EVERYTHING
```

The agent doesn't remember adding items. It just executes whatever task it receives.

## Architectural Benefits

### 1. Fault Isolation

```
┌──────────────┐
│ data_agent   │ ← Crashes
└──────────────┘
       ↓ (failure message)
┌──────────────┐
│ Supervisor   │ ← Handles error, continues
└──────────────┘
       ↓
┌──────────────┐
│ math_agent   │ ← Still works fine
└──────────────┘
```

If one agent crashes:
- Other agents are unaffected
- Supervisor can retry or fail gracefully
- No system-wide crash

### 2. No Race Conditions

**No shared memory = No locks/mutexes/races**

```rust
// Traditional approach (problematic)
struct SharedState {
    data: Arc<Mutex<HashMap<String, Value>>>  // Needs locks!
}

// Actor approach (safe)
// Each agent has its own state, communicate via messages
// No locks needed!
```

### 3. Clear Boundaries

From `src/actors/specialized_agent.rs:1-8`:

```rust
//! Information Hiding:
//! - Hides specific tool sets from coordinator
//! - Encapsulates domain-specific prompts
//! - Internal ReAct loop implementation hidden
//! - Exposes simple task execution interface
```

**Supervisor Knows:**
- Agent name: `"data_agent"`
- Agent description: `"Manages inventory data..."`
- Interface: `execute_task(task: &str) -> AgentResponse`

**Supervisor Doesn't Know:**
- What tools the agent has
- How the agent thinks (ReAct pattern)
- Agent's system prompts
- Tool execution details

**Agent Doesn't Know:**
- It's being orchestrated
- Other agents exist
- Overall task structure

### 4. Scalability

The architecture supports distribution:

```
┌──────────────┐          ┌──────────────┐
│ Supervisor   │          │ data_agent   │
│ (Server A)   │ ─ RPC ─> │ (Server B)   │
└──────────────┘          └──────────────┘
       │
       │ RPC
       ↓
┌──────────────┐
│ math_agent   │
│ (Server C)   │
└──────────────┘
```

Currently agents are in-process, but the message-passing design allows:
- Agents on different servers
- Agents in different processes
- Agents in different containers
- Load balancing across agent pools

### 5. Reproducibility

**Same input = Same output** (given same LLM responses)

No hidden state means:
- Easier debugging
- Deterministic testing
- Audit trails (all messages logged)

## Complete Example Visualization

Here's the full flow from your example:

```
User: "Add 100 units → count inventory → calculate 25%"
    ↓
┌─────────────────────────────────────────────────────────────┐
│ Supervisor (Orchestration Layer)                            │
│                                                              │
│ Orchestration History:                                      │
│ []                                                           │
│                                                              │
│ Conversation with LLM:                                      │
│ System: "You are a supervisor..."                           │
│ User: "Task: Add 100 units → count → calculate 25%"         │
│                                                              │
│ → LLM Planning Decision:                                    │
│   "Use data_agent to add items"                             │
└──────────────────────┬──────────────────────────────────────┘
                       │ Invoke: data_agent
                       ↓
┌─────────────────────────────────────────────────────────────┐
│ data_agent (First Invocation)                               │
│                                                              │
│ Tools: [add_item, search_items, count_items]                │
│                                                              │
│ ReAct Loop:                                                  │
│ 1. Think: "I should use add_item tool"                      │
│ 2. Act: add_item(name="Premium Widget", quantity=100, ...)  │
│ 3. Observe: "Added 100 units of 'Premium Widget'..."        │
│ 4. Think: "Task complete"                                   │
│ 5. Return: SUCCESS                                           │
│                                                              │
│ [Memory discarded after return]                             │
└──────────────────────┬──────────────────────────────────────┘
                       │ Result: "Added 100 units"
                       ↓
┌─────────────────────────────────────────────────────────────┐
│ Supervisor                                                   │
│                                                              │
│ Orchestration History:                                      │
│ [Step 1: data_agent added items → "Added 100 units"]        │
│                                                              │
│ Updated Conversation:                                        │
│ System: "You are a supervisor..."                           │
│ User: "Task: Add 100 units → count → calculate 25%"         │
│ Assistant: {"agent": "data_agent", "task": "add items"}     │
│ User: "Result: Added 100 units. What next?"                 │
│                                                              │
│ → LLM Planning Decision:                                    │
│   "Use data_agent to count inventory"                       │
└──────────────────────┬──────────────────────────────────────┘
                       │ Invoke: data_agent (new call)
                       ↓
┌─────────────────────────────────────────────────────────────┐
│ data_agent (Second Invocation - Fresh Start)                │
│                                                              │
│ Tools: [add_item, search_items, count_items]                │
│                                                              │
│ ReAct Loop:                                                  │
│ 1. Think: "I should use count_items tool"                   │
│ 2. Act: count_items()                                        │
│ 3. Observe: "Total items in inventory: 15"                  │
│ 4. Think: "Task complete"                                   │
│ 5. Return: SUCCESS                                           │
│                                                              │
│ [Memory discarded after return]                             │
│ [NO MEMORY of first call!]                                  │
└──────────────────────┬──────────────────────────────────────┘
                       │ Result: "Total: 15"
                       ↓
┌─────────────────────────────────────────────────────────────┐
│ Supervisor                                                   │
│                                                              │
│ Orchestration History:                                      │
│ [Step 1: data_agent added items → "Added 100 units"]        │
│ [Step 2: data_agent counted → "Total: 15"]                  │
│                                                              │
│ Updated Conversation:                                        │
│ ... (previous messages) ...                                 │
│ User: "Result: Total: 15. What next?"                       │
│                                                              │
│ → LLM Planning Decision:                                    │
│   "Use math_agent to calculate 25% of 15"                   │
└──────────────────────┬──────────────────────────────────────┘
                       │ Invoke: math_agent
                       ↓
┌─────────────────────────────────────────────────────────────┐
│ math_agent (First Invocation)                               │
│                                                              │
│ Tools: [calculate, percentage]                              │
│                                                              │
│ ReAct Loop:                                                  │
│ 1. Think: "I should use percentage tool"                    │
│ 2. Act: percentage(value=15, percent=25)                    │
│ 3. Observe: "25% of 15 = 3"                                 │
│ 4. Think: "Task complete"                                   │
│ 5. Return: SUCCESS                                           │
│                                                              │
│ [Memory discarded after return]                             │
└──────────────────────┬──────────────────────────────────────┘
                       │ Result: "25% of 15 = 3"
                       ↓
┌─────────────────────────────────────────────────────────────┐
│ Supervisor                                                   │
│                                                              │
│ Orchestration History:                                      │
│ [Step 1: data_agent added items → "Added 100 units"]        │
│ [Step 2: data_agent counted → "Total: 15"]                  │
│ [Step 3: math_agent calculated → "25% of 15 = 3"]           │
│                                                              │
│ Updated Conversation:                                        │
│ ... (previous messages) ...                                 │
│ User: "Result: 25% of 15 = 3. What next?"                   │
│                                                              │
│ → LLM Planning Decision:                                    │
│   "is_complete: true"                                        │
│   "final_answer: All tasks completed. Added items, ..."     │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ↓
          Return to User with Final Result
```

## Key Insights

### 1. Sequential Orchestration

The supervisor works **sequentially**, not in parallel:
- Waits for each agent to complete
- Plans next action based on previous results
- Can invoke same agent multiple times

### 2. Stateless Agents

Each agent invocation is **completely independent**:
- No memory of previous calls
- Fresh ReAct loop every time
- Only knows its tools and current task

### 3. Supervisor as Memory Keeper

Only the supervisor maintains context:
- Tracks all agent invocations
- Remembers results
- Combines information across steps

### 4. Information Hiding in Practice

**Module boundaries are strict:**
- Supervisor: Orchestration logic
- Agent: Task execution with tools
- Tool: Specific capability implementation

Changes to one don't affect others as long as interfaces are stable.

## Design Patterns Demonstrated

### 1. Actor Model
- Isolated processes communicating via messages
- No shared state
- Location transparency (could be distributed)

### 2. ReAct Pattern (Reason + Act)
- Think: Plan next action
- Act: Execute tool
- Observe: See result
- Repeat until complete

### 3. Supervisor Pattern
- Hierarchical coordination
- Fault tolerance
- Task decomposition

### 4. Information Hiding (Parnas)
- Implementation details hidden
- Clean interfaces
- Independent evolution

## Performance Characteristics

### Sequential Execution
```
Total Time = Sum of agent execution times
Task 1: 3.5 seconds (data_agent: add items)
Task 2: 2.3 seconds (data_agent: count)
Task 3: 1.6 seconds (math_agent: calculate)
Total: 7.4 seconds + orchestration overhead
```

### Trade-offs

**Pros:**
- Simpler reasoning (no concurrency bugs)
- Clear audit trail
- Easier debugging
- Results from one agent can inform next

**Cons:**
- Slower than parallel execution
- Agents wait idle while others work
- Sequential bottleneck

### Future: Parallel Orchestration

The architecture could support parallel execution:

```rust
// Current: Sequential
let result1 = agent1.execute_task(task1).await;
let result2 = agent2.execute_task(task2).await;

// Future: Parallel (if tasks are independent)
let (result1, result2) = tokio::join!(
    agent1.execute_task(task1),
    agent2.execute_task(task2)
);
```

This would require:
- Supervisor to detect independent tasks
- Parallel planning strategy
- Result synchronization

## Two Types of "Memory" in LLM Fusion

It's important to distinguish between two different concepts of memory in this system:

### 1. Agent Memory (What This Document Explains)

**Agents DO NOT share memory with each other.**

```
┌──────────────┐      ┌──────────────┐
│ data_agent   │      │ math_agent   │
│              │  ✗   │              │
│ No shared    │ ───> │ No shared    │
│ memory       │      │ memory       │
└──────────────┘      └──────────────┘
```

- Agents are stateless between invocations
- Each agent execution is independent
- Supervisor maintains context, not agents
- Communication only via messages

**Example:**
```rust
// First call
data_agent.execute_task("add 100 items")
  → completes → FORGETS EVERYTHING

// Second call
data_agent.execute_task("count items")
  → starts fresh → NO MEMORY of first call
```

### 2. Conversation Memory (User Sessions)

**Users CAN have persistent conversation memory.**

This is a completely separate feature for user-facing applications where you want to remember conversation history across sessions.

```
Session 1 (Today):
User: "My name is Alice"
AI: "Nice to meet you, Alice!"
[Saved to disk]

Session 2 (Tomorrow):
User: "What's my name?"
AI: "Your name is Alice!" ← Remembers from previous session
```

**Implementation:** `src/api.rs:641-781` - Session API

```rust
use llm_fusion::api::session::{self, StorageType};

// Create persistent session
let mut session = session::create_session(
    "user-123",
    StorageType::FileSystem(PathBuf::from("./sessions"))
).await?;

// Conversation is saved automatically
session.send_message("Remember: I like blue").await?;

// Later (even after restart)
let mut session = session::create_session(
    "user-123",  // Same session ID
    StorageType::FileSystem(PathBuf::from("./sessions"))
).await?;

// Conversation history is loaded from disk
session.send_message("What color do I like?").await?;
// → "You like blue!"
```

**CLI Usage:**

```bash
# Without memory (default) - ephemeral
llm-fusion interactive

# With persistent memory
llm-fusion interactive --memory --session-id alice

# Resume same session later
llm-fusion interactive --memory --session-id alice
# ↑ Remembers previous conversations
```

**Storage Options:**
- **Memory**: `StorageType::Memory` - Lost when process ends
- **FileSystem**: `StorageType::FileSystem(path)` - Persists to disk

**Special Commands** (in interactive mode with `--memory`):
- `/clear` - Clear session history
- `/count` - Show message count
- `/help` - Show available commands

### Key Distinction

| Aspect | Agent Memory | Conversation Memory |
|--------|--------------|---------------------|
| **What** | Agent internal state | User conversation history |
| **Scope** | Within single task execution | Across multiple sessions |
| **Persistence** | Never (always stateless) | Optional (memory/filesystem) |
| **Purpose** | Isolation & reliability | User experience |
| **Location** | `src/actors/specialized_agent.rs` | `src/api.rs` + `src/storage/` |
| **Who uses** | Supervisor orchestrating agents | End users chatting with AI |

**Simple Rule:**
- **Agents forget everything** between calls (by design for reliability)
- **Users can remember** conversations (optional feature for UX)

## Summary

**Agent Memory Sharing:** None. Pure message passing.

**User Conversation Memory:** Optional persistent storage.

**Orchestration:** Supervisor plans → delegates → collects → repeats.

**Key Benefit:** Reliability through isolation.

**Key Trade-off:** Sequential execution for simpler reasoning.

This architecture prioritizes **correctness, maintainability, and fault tolerance** over raw performance. For most LLM agent tasks (where LLM latency dominates), this is the right choice.


Exactly! You have two different multi-agent modes plus several other patterns:

  1. Router Agent (One-Way Ticket)

  File: src/actors/router_agent.rs

  Pattern: Intent classification → Route to ONE agent → Agent completes task → Done

  use llm_fusion::{init, router};

  init().await?;
  let result = router::route_task("Create a file called hello.txt").await?;

  How it works:
  - Router's LLM analyzes the task
  - Picks the BEST agent for the job
  - Sends task to that ONE agent
  - Agent executes and returns
  - One-way ticket: Agent doesn't come back

  Best for: Single-domain tasks with clear intent (file ops, shell commands, web requests)

  2. Supervisor Agent (Return Ticket)

  File: src/actors/supervisor_agent.rs

  Pattern: Decompose task → Delegate to agents → Collect results → Repeat until complete

  use llm_fusion::{init, supervisor};

  init().await?;
  let result = supervisor::orchestrate(
      "List all Rust files, count them, write count to file"
  ).await?;

  How it works:
  - Supervisor's LLM breaks down complex task
  - Invokes agents sequentially
  - Can call SAME agent multiple times
  - Combines results from multiple agents
  - Return ticket: Agents can be called again

  Best for: Multi-step tasks spanning multiple domains

  Comparison

  | Aspect            | Router                      | Supervisor                              |
  |-------------------|-----------------------------|-----------------------------------------|
  | Complexity        | Simple                      | Complex                                 |
  | Agent invocations | 1 agent, 1 time             | Multiple agents, multiple times         |
  | Pattern           | One-way ticket              | Return ticket                           |
  | Task type         | Single-domain               | Multi-domain                            |
  | Orchestration     | None (direct routing)       | Full orchestration                      |
  | Example           | "Create a file"             | "Fetch data, save to file, count lines" |
  | LLM calls         | 1 (routing) + agent's calls | Multiple (planning) + agent calls       |

  Other Modes (For Completeness)

  3. Simple Agent

  File: src/actors/agent_actor.rs

  use llm_fusion::agent;
  let result = agent::run_task("List files").await?;
  - No routing, no orchestration
  - Just a general-purpose agent with all tools
  - Fastest for simple tasks

  4. Custom Agent with Tools

  File: src/api.rs:334-364

  use llm_fusion::agent;
  let tools = vec![Arc::new(MyCustomTool::new())];
  let result = agent::run_task_with_tools(tools, "Use my tool").await?;
  - Single agent with your custom tools
  - No routing or orchestration

  5. Agent Session (Stateful)

  File: src/actors/agent_session.rs

  use llm_fusion::api::session;
  let mut session = session::create_session("user-123", StorageType::FileSystem(...)).await?;
  let result = session.send_message("Remember this").await?;
  // Later...
  let result = session.send_message("What did I say?").await?;
  - Persistent conversation memory
  - Remembers context across sessions

  Summary

  Your two main multi-agent orchestration modes are:

  1. Router - One agent, one shot (one-way ticket)
  2. Supervisor - Multiple agents, multiple times (return ticket)

  Choose based on task complexity:
  - Simple, single-domain → Router
  - Complex, multi-step → Supervisor
