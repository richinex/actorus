# Factory Refactor: AgentBuilder as Foundation

## Overview

The `specialized_agents_factory` has been refactored to use `AgentBuilder` as its internal implementation. This demonstrates proper abstraction layers where the factory is now a thin convenience layer over the flexible builder API.

## Before vs After

### Before: Manual Configuration
```rust
pub fn create_file_ops_agent(settings: Settings, api_key: String) -> SpecializedAgent {
    let config = SpecializedAgentConfig {
        name: "file_ops_agent".to_string(),
        description: "Handles file system operations...".to_string(),
        system_prompt: "You are a file operations specialist...".to_string(),
        tools: vec![
            Arc::new(filesystem::ReadFileTool::new(1024 * 1024 * 10)),
            Arc::new(filesystem::WriteFileTool::new(1024 * 1024 * 10)),
        ],
    };

    SpecializedAgent::new(config, settings, api_key)
}
```

**Issues**:
- Manual `Arc::new()` wrapping
- Manual `.to_string()` calls
- Verbose vector construction
- Code duplication across all factory functions

### After: AgentBuilder Foundation
```rust
pub fn create_file_ops_agent(settings: Settings, api_key: String) -> SpecializedAgent {
    let (name, description, system_prompt, tools) = AgentBuilder::new("file_ops_agent")
        .description(
            "Handles file system operations including reading and writing files. \
             Use this agent for tasks involving file I/O operations."
        )
        .system_prompt(
            "You are a file operations specialist. Your role is to handle file system tasks. \
             You can read files, write files, and manage file contents. \
             Focus on providing accurate file operations and clear feedback about what was done."
        )
        .tool(filesystem::ReadFileTool::new(1024 * 1024 * 10))
        .tool(filesystem::WriteFileTool::new(1024 * 1024 * 10))
        .build();

    let config = SpecializedAgentConfig {
        name,
        description,
        system_prompt,
        tools,
    };

    SpecializedAgent::new(config, settings, api_key)
}
```

**Benefits**:
- No manual `Arc::new()` - builder handles it
- No manual `.to_string()` - builder handles it
- Fluent, readable API
- Same abstraction used by users and internally

## Architecture Layers

```
┌─────────────────────────────────────────────────┐
│  User-Facing API (examples, applications)       │
├─────────────────────────────────────────────────┤
│  Factory Functions (convenience layer)          │ ← Curated recipes
│  - create_file_ops_agent()                      │
│  - create_shell_agent()                         │
│  - create_web_agent()                           │
│  - create_general_agent()                       │
├─────────────────────────────────────────────────┤
│  AgentBuilder (flexible configuration)          │ ← Core abstraction
│  - .new()                                       │
│  - .description()                               │
│  - .system_prompt()                             │
│  - .tool()                                      │
│  - .build()                                     │
├─────────────────────────────────────────────────┤
│  SpecializedAgent (domain logic)                │ ← Implementation
└─────────────────────────────────────────────────┘
```

## Benefits of This Refactor

### 1. **Single Source of Truth**
AgentBuilder is the **only** way to configure agents. The factory doesn't duplicate configuration logic.

### 2. **Consistency**
Users and internal code use the same API. No "do as I say, not as I do" anti-pattern.

### 3. **Maintainability**
Changes to AgentBuilder automatically benefit both:
- User-defined custom agents
- Built-in factory agents

### 4. **Reduced Code Duplication**
Before: Manual Arc wrapping in 4 places
After: Arc wrapping in 1 place (AgentBuilder)

### 5. **Information Hiding (Parnas Principle)**
- **Factory hides**: Specific tool configurations, timeouts, size limits
- **AgentBuilder hides**: Arc wrapping, String conversion, default values
- **SpecializedAgent hides**: ReAct loop, tool execution, LLM interaction

Each layer only exposes what's necessary to the layer above.

## Usage Patterns

### Pattern 1: Factory (Quick Start)
```rust
// Zero configuration - use pre-made agents
let agents = specialized_agents_factory::create_default_agents(settings, api_key);
router::route_task("list files").await?;
```

**Use When**: You want standard file/shell/web agents with sensible defaults

### Pattern 2: AgentBuilder (Custom Agents)
```rust
// Full control - build your own agents
let weather_agent = AgentBuilder::new("weather_agent")
    .description("Handles weather queries")
    .system_prompt("You are a weather specialist")
    .tool(GetWeatherTool::new())
    .tool(GetForecastTool::new());

let agents = AgentCollection::new()
    .add(weather_agent);

router::route_task_with_custom_agents(agents.build(), "weather?").await?;
```

**Use When**: You need domain-specific agents (weather, email, calendar, etc.)

### Pattern 3: Hybrid (Mix Both)
```rust
// Use factory for standard agents, builder for custom ones
let file_agent = create_file_ops_agent(settings.clone(), api_key.clone());
let weather_agent = AgentBuilder::new("weather_agent")
    .tool(GetWeatherTool::new())
    .build_agent(settings.clone(), api_key.clone());

let agents = vec![file_agent, weather_agent];
```

**Use When**: You need both built-in and custom agents together

## Implementation Details

### AgentBuilder Output
```rust
.build() -> (String, String, String, Vec<Arc<dyn Tool>>)
         //  ^name   ^desc    ^prompt  ^tools
```

This tuple is exactly what `SpecializedAgentConfig` expects, making the integration seamless.

### Factory Responsibilities
1. **Choose tool configurations**: File size limits, timeouts, etc.
2. **Define agent personalities**: System prompts and descriptions
3. **Expose simple interface**: `create_X_agent(settings, api_key) -> Agent`

### AgentBuilder Responsibilities
1. **Fluent configuration API**: Method chaining for readability
2. **Handle conversions**: String, Arc wrapping
3. **Provide defaults**: Description and system prompt auto-generation
4. **Validate configuration**: Ensure required fields are present

## Testing

All 30 tests pass after refactor:
```
test result: ok. 30 passed; 0 failed; 0 ignored
```

This includes:
- AgentBuilder tests (basic, defaults, collection)
- Storage tests (memory, filesystem)
- Tool tests (filesystem, shell, http)
- Registry tests
- Executor tests

## Backward Compatibility

✅ **100% backward compatible**

All existing code using the factory continues to work:
- `router::route_task()` - Uses factory agents
- `supervisor::orchestrate()` - Uses factory agents
- Examples: `router_usage.rs`, `supervisor_usage.rs` - Unchanged

New code can use AgentBuilder:
- `router::route_task_with_custom_agents()` - Uses builder agents
- `supervisor::orchestrate_with_custom_agents()` - Uses builder agents
- Examples: `router_custom_agents.rs`, `supervisor_custom_agents.rs`

## Design Principles Applied

### 1. **DRY (Don't Repeat Yourself)**
AgentBuilder centralizes configuration logic.

### 2. **Single Responsibility**
- Factory: Curate default agents
- Builder: Flexible agent configuration
- SpecializedAgent: Agent execution logic

### 3. **Information Hiding (Parnas)**
Each module hides implementation details:
```
Factory hides → tool limits, timeouts
Builder hides → Arc wrapping, conversions
Agent hides   → ReAct loop, LLM calls
```

### 4. **Open/Closed Principle**
Open for extension (add new agents via builder), closed for modification (factory provides stable defaults).

### 5. **Composition Over Inheritance**
Factory **uses** builder rather than duplicating its logic.

## File Changes

**Modified**: `src/actors/specialized_agents_factory.rs`
- Added `use crate::actors::agent_builder::AgentBuilder;`
- Refactored all 4 factory functions to use builder
- Added documentation explaining layer relationship

**No Breaking Changes**: All public APIs unchanged

## Conclusion

This refactor demonstrates **proper abstraction**:

1. **AgentBuilder** = Flexible foundation (library core)
2. **Factory** = Convenience layer (curated defaults)
3. **Users** = Choice of quick start (factory) or customization (builder)

The factory is now what it should be: a **thin convenience layer** that showcases best practices while internally using the same powerful API available to users.

This is real abstraction - not just moving code around, but creating meaningful layers where each level:
- Hides appropriate details
- Exposes useful interfaces
- Builds on the layer below
- Provides value to the layer above

**"Make the easy things easy and the hard things possible."** ✅
