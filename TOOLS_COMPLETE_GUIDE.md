# Complete Tool System Guide

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Core Concepts](#core-concepts)
4. [Tool Trait](#tool-trait)
5. [Creating Tools - Three Approaches](#creating-tools---three-approaches)
6. [Tool Registry](#tool-registry)
7. [Tool Execution](#tool-execution)
8. [Macro System Deep Dive](#macro-system-deep-dive)
9. [Best Practices](#best-practices)
10. [Complete Examples](#complete-examples)

---

## Overview

The LLM Fusion tool system provides a **flexible, extensible framework** for creating tools that LLM agents can use. Tools are self-describing functions that agents can discover, validate, and execute dynamically.

### What is a Tool?

A **tool** is a capability that an agent can use to interact with the outside world:
- File operations (read, write, append)
- Shell commands
- HTTP requests
- Custom business logic

Each tool is:
- **Self-describing**: Provides metadata about what it does and what parameters it accepts
- **Validated**: Checks parameters before execution
- **Async**: Supports asynchronous operations
- **Error-safe**: Returns structured results, not panics

### Design Principles

1. **Information Hiding**: Tool internals are hidden behind the `Tool` trait
2. **Type Safety**: Compile-time checks via Rust's type system
3. **Extensibility**: Easy to add new tools without modifying core code
4. **Consistency**: Standardized patterns across all tools

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Agent System                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │            Tool Registry                          │  │
│  │  - Maintains map of tool name → Tool instance    │  │
│  │  - Provides tool discovery                        │  │
│  │  - Handles tool execution                         │  │
│  └───────────────────────────────────────────────────┘  │
│                         │                               │
│                         ▼                               │
│  ┌───────────────────────────────────────────────────┐  │
│  │              Tool Trait                           │  │
│  │  - metadata() → ToolMetadata                      │  │
│  │  - validate(args) → Result<()>                    │  │
│  │  - execute(args) → Result<ToolResult>             │  │
│  └───────────────────────────────────────────────────┘  │
│                         │                               │
│         ┌───────────────┼───────────────┐              │
│         ▼               ▼               ▼              │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐         │
│  │ ReadFile │    │WriteFile │    │ShellTool │  ...    │
│  │   Tool   │    │   Tool   │    │          │         │
│  └──────────┘    └──────────┘    └──────────┘         │
└─────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. Agent receives task → "read file config.json"

2. Agent queries ToolRegistry → get_tool("read_file")

3. ToolRegistry returns ReadFileTool instance

4. Agent calls tool.metadata() → gets parameters schema

5. Agent constructs JSON args → {"path": "config.json"}

6. Agent calls tool.validate(args) → checks parameters

7. Agent calls tool.execute(args) → reads file

8. Tool returns ToolResult → {success: true, output: "..."}

9. Agent processes result → continues reasoning
```

---

## Core Concepts

### 1. ToolMetadata

Describes what a tool does and what parameters it accepts.

```rust
pub struct ToolMetadata {
    pub name: String,              // Unique tool identifier
    pub description: String,       // Human-readable description
    pub parameters: Vec<ToolParameter>,  // List of parameters
}
```

**Example:**
```rust
ToolMetadata {
    name: "read_file".to_string(),
    description: "Read the contents of a file".to_string(),
    parameters: vec![
        ToolParameter {
            name: "path".to_string(),
            param_type: "string".to_string(),
            description: "The file path to read".to_string(),
            required: true,
        }
    ],
}
```

### 2. ToolParameter

Describes a single parameter.

```rust
pub struct ToolParameter {
    pub name: String,          // Parameter name
    pub param_type: String,    // Type: "string", "number", "boolean"
    pub description: String,   // What the parameter does
    pub required: bool,        // Is it required?
}
```

### 3. ToolResult

The result of tool execution.

```rust
pub struct ToolResult {
    pub success: bool,         // Did it succeed?
    pub output: String,        // Success output
    pub error: Option<String>, // Error message if failed
}
```

**Creating results:**
```rust
// Success
ToolResult::success("File read successfully")

// Failure
ToolResult::failure("File not found")
```

---

## Tool Trait

The core abstraction for all tools.

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get tool metadata
    fn metadata(&self) -> ToolMetadata;

    /// Validate arguments (optional)
    fn validate(&self, args: &Value) -> Result<()> {
        Ok(())
    }

    /// Execute the tool
    async fn execute(&self, args: Value) -> Result<ToolResult>;
}
```

### Method Responsibilities

| Method | Purpose | When Called |
|--------|---------|-------------|
| `metadata()` | Describe the tool | Discovery, help systems |
| `validate()` | Check parameters | Before execution |
| `execute()` | Perform the action | After validation |

### Implementation Requirements

1. **`Send + Sync`**: Tools must be thread-safe (required for async)
2. **`metadata()`**: Must return consistent metadata
3. **`validate()`**: Should check all required parameters
4. **`execute()`**: Should call `validate()` first, then perform action

---

## Creating Tools - Three Approaches

### Approach 1: Manual (Full Control)

**When to use:** Learning, debugging, complex custom logic

**Pros:**
- Full control over every detail
- Easy to understand what's happening
- No macro magic

**Cons:**
- Most verbose (58 lines for a simple tool)
- Repetitive boilerplate
- More opportunity for errors

**Example:**

```rust
pub struct ReadFileTool {
    max_size_bytes: usize,
}

impl ReadFileTool {
    pub fn new(max_size_bytes: usize) -> Self {
        Self { max_size_bytes }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "read_file".to_string(),
            description: "Read the contents of a file".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "The file path to read".to_string(),
                    required: true,
                },
            ],
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'path' must be a string"))?;

        if path.is_empty() {
            return Err(anyhow::anyhow!("Path cannot be empty"));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let path = args["path"].as_str().unwrap();

        match fs::read_to_string(path).await {
            Ok(contents) => Ok(ToolResult::success(contents)),
            Err(e) => Ok(ToolResult::failure(format!("Failed: {}", e))),
        }
    }
}
```

### Approach 2: Declarative Macros (Most Flexible)

**When to use:** Production code, complex tools with validation

**Pros:**
- 41% less code than manual
- Maintains full flexibility
- Easy to add custom validation
- No procedural macro complexity

**Cons:**
- Multiple macros to remember
- Still requires some boilerplate

**Available Macros:**
- `tool_metadata!` - Generate metadata
- `validate_required_string!` - Validate required strings
- `validate_optional_string!` - Validate optional strings with defaults
- `validate_required_number!` - Validate required numbers
- `tool_result!` - Create results

**Example:**

```rust
use llm_fusion::{tool_metadata, validate_required_string, tool_result};

pub struct ReadFileTool {
    max_size_bytes: usize,
}

impl ReadFileTool {
    pub fn new(max_size_bytes: usize) -> Self {
        Self { max_size_bytes }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn metadata(&self) -> ToolMetadata {
        tool_metadata! {
            name: "read_file",
            description: "Read the contents of a file",
            parameters: [
                {
                    name: "path",
                    type: "string",
                    description: "The file path to read",
                    required: true
                }
            ]
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let path = validate_required_string!(args, "path");

        if path.is_empty() {
            return Err(anyhow::anyhow!("Path cannot be empty"));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let path = validate_required_string!(args, "path");

        match fs::read_to_string(path).await {
            Ok(contents) => tool_result!(success: contents),
            Err(e) => tool_result!(failure: format!("Failed: {}", e)),
        }
    }
}
```

### Approach 3: Procedural Macro (Cleanest)

**When to use:** Simple tools, rapid prototyping, maximum cleanliness

**Pros:**
- 50% less code than manual
- Single annotation
- Auto-generates metadata helper
- Cleanest syntax

**Cons:**
- Only generates metadata helper
- Still need to implement validate/execute
- Rust-analyzer may show false diagnostics

**Example:**

```rust
use llm_fusion::tool;
use llm_fusion::{validate_required_string, tool_result};

#[tool(name = "read_file", description = "Read the contents of a file")]
pub struct ReadFileTool {
    max_size_bytes: usize,
}

impl ReadFileTool {
    pub fn new(max_size_bytes: usize) -> Self {
        Self { max_size_bytes }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn metadata(&self) -> ToolMetadata {
        Self::tool_metadata()  // Auto-generated by #[tool!]
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let path = validate_required_string!(args, "path");

        if path.is_empty() {
            return Err(anyhow::anyhow!("Path cannot be empty"));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let path = validate_required_string!(args, "path");

        match fs::read_to_string(path).await {
            Ok(contents) => tool_result!(success: contents),
            Err(e) => tool_result!(failure: format!("Failed: {}", e)),
        }
    }
}
```

### Comparison Table

| Aspect | Manual | Declarative Macros | Proc Macro |
|--------|--------|-------------------|------------|
| **Lines of code** | 58 | 34 (-41%) | 29 (-50%) |
| **Readability** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Flexibility** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Learning curve** | Easy | Medium | Medium |
| **Best for** | Learning | Production | Simple tools |

---

## Tool Registry

The registry maintains all available tools and provides discovery.

### Structure

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}
```

### Operations

```rust
impl ToolRegistry {
    /// Create empty registry
    pub fn new() -> Self;

    /// Create with default tools
    pub fn with_defaults() -> Self;

    /// Register a tool
    pub fn register(&mut self, tool: Arc<dyn Tool>);

    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>>;

    /// List all tool names
    pub fn list_tools(&self) -> Vec<String>;

    /// Get all tools metadata
    pub fn all_metadata(&self) -> Vec<ToolMetadata>;

    /// Get formatted description
    pub fn tools_description(&self) -> String;
}
```

### Usage Example

```rust
// Create registry with defaults
let mut registry = ToolRegistry::with_defaults();

// Register custom tool
registry.register(Arc::new(MyCustomTool::new()));

// Get tool
let tool = registry.get_tool("read_file").unwrap();

// Execute
let result = tool.execute(json!({"path": "test.txt"})).await?;
```

### Default Tools

The `with_defaults()` method registers:

| Tool | Description |
|------|-------------|
| `execute_shell` | Execute shell commands |
| `read_file` | Read file contents |
| `write_file` | Write to files |
| `append_file` | Append to files |
| `http_request` | Make HTTP requests |

---

## Tool Execution

### Execution Flow

```
┌─────────────────────────────────────────────┐
│ 1. Agent constructs arguments               │
│    args = {"path": "config.json"}           │
└────────────────┬────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│ 2. Get tool from registry                   │
│    tool = registry.get_tool("read_file")    │
└────────────────┬────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│ 3. Validate arguments                       │
│    tool.validate(&args)?                    │
│    - Check required params                  │
│    - Check types                            │
│    - Custom validation                      │
└────────────────┬────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│ 4. Execute tool                             │
│    result = tool.execute(args).await?       │
│    - Perform actual operation               │
│    - Handle errors gracefully               │
└────────────────┬────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│ 5. Process result                           │
│    if result.success {                      │
│        use result.output                    │
│    } else {                                 │
│        handle result.error                  │
│    }                                        │
└─────────────────────────────────────────────┘
```

### Error Handling

Tools have **two layers** of error handling:

**1. Validation Errors** (return `Err`):
```rust
fn validate(&self, args: &Value) -> Result<()> {
    let path = validate_required_string!(args, "path");

    if path.is_empty() {
        return Err(anyhow::anyhow!("Path cannot be empty"));
    }

    Ok(())
}
```

**2. Execution Errors** (return `ToolResult::failure`):
```rust
async fn execute(&self, args: Value) -> Result<ToolResult> {
    self.validate(&args)?;  // Validation error propagates

    match fs::read_to_string(path).await {
        Ok(contents) => Ok(ToolResult::success(contents)),
        Err(e) => Ok(ToolResult::failure(format!("File error: {}", e))),
    }
}
```

**Why two layers?**
- **Validation errors**: Parameter problems (agent should fix)
- **Execution errors**: Runtime problems (agent should handle)

---

## Macro System Deep Dive

### Declarative Macros

Defined in `src/tools/macros.rs`. These are compile-time text substitutions.

#### 1. `tool_metadata!`

**Definition:**
```rust
#[macro_export]
macro_rules! tool_metadata {
    (
        name: $name:expr,
        description: $description:expr,
        parameters: [
            $(
                {
                    name: $param_name:expr,
                    type: $param_type:expr,
                    description: $param_desc:expr,
                    required: $param_required:expr
                }
            ),* $(,)?
        ]
    ) => {
        $crate::tools::ToolMetadata {
            name: $name.to_string(),
            description: $description.to_string(),
            parameters: vec![
                $(
                    $crate::tools::ToolParameter {
                        name: $param_name.to_string(),
                        param_type: $param_type.to_string(),
                        description: $param_desc.to_string(),
                        required: $param_required,
                    }
                ),*
            ],
        }
    };
}
```

**How it works:**
1. Match the input pattern
2. Extract name, description, parameters
3. Generate ToolMetadata struct
4. Convert all strings with `.to_string()`

**Expansion example:**
```rust
// Input:
tool_metadata! {
    name: "greet",
    description: "Greets a person",
    parameters: [
        { name: "name", type: "string", description: "Person's name", required: true }
    ]
}

// Expands to:
ToolMetadata {
    name: "greet".to_string(),
    description: "Greets a person".to_string(),
    parameters: vec![
        ToolParameter {
            name: "name".to_string(),
            param_type: "string".to_string(),
            description: "Person's name".to_string(),
            required: true,
        }
    ],
}
```

#### 2. `validate_required_string!`

**Definition:**
```rust
#[macro_export]
macro_rules! validate_required_string {
    ($args:expr, $param:expr) => {
        $args[$param]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'{}' parameter is required and must be a string", $param))?
    };
}
```

**How it works:**
1. Index into args JSON with parameter name
2. Try to convert to string with `.as_str()`
3. If None, create error with parameter name
4. `?` propagates error if conversion fails

**Expansion example:**
```rust
// Input:
let path = validate_required_string!(args, "path");

// Expands to:
let path = args["path"]
    .as_str()
    .ok_or_else(|| anyhow::anyhow!("'{}' parameter is required and must be a string", "path"))?;
```

#### 3. `validate_optional_string!`

**Definition:**
```rust
#[macro_export]
macro_rules! validate_optional_string {
    ($args:expr, $param:expr, $default:expr) => {
        $args[$param]
            .as_str()
            .unwrap_or($default)
    };
}
```

**How it works:**
1. Try to get string from args
2. If missing or not a string, use default
3. No error - just returns default

#### 4. `tool_result!`

**Definition:**
```rust
#[macro_export]
macro_rules! tool_result {
    (success: $msg:expr) => {
        Ok($crate::tools::ToolResult::success($msg))
    };
    (failure: $msg:expr) => {
        Ok($crate::tools::ToolResult::failure($msg))
    };
}
```

**How it works:**
1. Match `success:` or `failure:` pattern
2. Call appropriate ToolResult method
3. Wrap in Ok() for Result type

### Procedural Macro

Defined in `llm_fusion_macros/src/lib.rs`. This is a compiler plugin.

#### How `#[tool!]` Works

**1. Parse attributes:**
```rust
#[proc_macro_attribute]
pub fn tool(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse: name = "greet", description = "..."
    let tool_args = parse_macro_input!(args as ToolArgs);

    // Parse: pub struct GreetTool;
    let input_item: syn::Item = parse_macro_input!(input);

    // ...
}
```

**2. Extract struct name:**
```rust
let struct_item = if let syn::Item::Struct(item_struct) = &input_item {
    item_struct
} else {
    // Error: can only apply to structs
};

let struct_name = &struct_item.ident;  // "GreetTool"
```

**3. Generate helper method:**
```rust
let expanded = quote! {
    #input_item  // Original struct

    impl #struct_name {
        pub fn tool_metadata() -> llm_fusion::tools::ToolMetadata {
            llm_fusion::tools::ToolMetadata {
                name: #tool_name.to_string(),
                description: #tool_desc.to_string(),
                parameters: vec![
                    // Generated parameters
                ],
            }
        }
    }
};
```

**4. Return generated code:**
```rust
TokenStream::from(expanded)
```

#### Complete Expansion

**Input:**
```rust
#[tool(name = "greet", description = "Greets a person")]
pub struct GreetTool;
```

**Output:**
```rust
pub struct GreetTool;

impl GreetTool {
    pub fn tool_metadata() -> llm_fusion::tools::ToolMetadata {
        llm_fusion::tools::ToolMetadata {
            name: "greet".to_string(),
            description: "Greets a person".to_string(),
            parameters: vec![],
        }
    }
}
```

---

## Best Practices

### 1. Validation Strategy

**Always validate in two places:**

```rust
fn validate(&self, args: &Value) -> Result<()> {
    // 1. Parameter existence and type
    let path = validate_required_string!(args, "path");

    // 2. Business logic validation
    if path.is_empty() {
        return Err(anyhow::anyhow!("Path cannot be empty"));
    }

    if !path.starts_with("/allowed/") {
        return Err(anyhow::anyhow!("Path not in allowed directory"));
    }

    Ok(())
}

async fn execute(&self, args: Value) -> Result<ToolResult> {
    self.validate(&args)?;  // Always call validate first

    // Now safe to unwrap
    let path = validate_required_string!(args, "path");
    // ...
}
```

### 2. Error Messages

**Be specific and actionable:**

❌ Bad:
```rust
return Err(anyhow::anyhow!("Error"));
```

✅ Good:
```rust
return Err(anyhow::anyhow!(
    "File '{}' exceeds maximum size of {} bytes (actual: {} bytes)",
    path, max_size, actual_size
));
```

### 3. Security

**Always validate paths:**

```rust
fn is_path_allowed(&self, path: &Path) -> bool {
    if let Some(ref allowed) = self.allowed_paths {
        allowed.iter().any(|allowed_path| {
            path.starts_with(allowed_path)
        })
    } else {
        true  // No restrictions
    }
}
```

**Limit resource usage:**

```rust
pub struct ReadFileTool {
    max_size_bytes: usize,  // Limit file size
}

pub struct ShellTool {
    timeout_secs: u64,  // Limit execution time
}
```

### 4. Tool Naming

**Use clear, consistent names:**

| Tool Type | Pattern | Example |
|-----------|---------|---------|
| Actions | verb_noun | `read_file`, `execute_shell` |
| Queries | get_noun | `get_status`, `get_config` |
| Mutations | set_noun | `set_config`, `set_mode` |

### 5. Documentation

**Always document:**

```rust
/// Read file tool
///
/// Reads the entire contents of a file into a string.
///
/// # Security
/// - Respects allowed_paths if configured
/// - Enforces max_size_bytes limit
///
/// # Parameters
/// - `path` (required): File path to read
///
/// # Returns
/// - Success: File contents as string
/// - Failure: Error message with details
pub struct ReadFileTool {
    allowed_paths: Option<Vec<PathBuf>>,
    max_size_bytes: usize,
}
```

### 6. Testing

**Test all edge cases:**

```rust
#[tokio::test]
async fn test_read_file_success() {
    let tool = ReadFileTool::new(1024);
    let result = tool.execute(json!({"path": "test.txt"})).await.unwrap();
    assert!(result.success);
}

#[tokio::test]
async fn test_read_file_missing() {
    let tool = ReadFileTool::new(1024);
    let result = tool.execute(json!({"path": "missing.txt"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("not found"));
}

#[tokio::test]
async fn test_read_file_size_limit() {
    let tool = ReadFileTool::new(10);  // Very small limit
    let result = tool.execute(json!({"path": "large.txt"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("too large"));
}
```

---

## Complete Examples

### Example 1: Simple Tool (No State)

```rust
use llm_fusion::tool;
use llm_fusion::{validate_required_string, tool_result};

#[tool(name = "uppercase", description = "Convert text to uppercase")]
pub struct UppercaseTool;

#[async_trait]
impl Tool for UppercaseTool {
    fn metadata(&self) -> ToolMetadata {
        Self::tool_metadata()
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let text = validate_required_string!(args, "text");
        let result = text.to_uppercase();
        tool_result!(success: result)
    }
}
```

### Example 2: Stateful Tool

```rust
use llm_fusion::{tool_metadata, validate_required_string, tool_result};

pub struct CacheTool {
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl CacheTool {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Tool for CacheTool {
    fn metadata(&self) -> ToolMetadata {
        tool_metadata! {
            name: "cache_set",
            description: "Store a value in cache",
            parameters: [
                { name: "key", type: "string", description: "Cache key", required: true },
                { name: "value", type: "string", description: "Value to store", required: true }
            ]
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let key = validate_required_string!(args, "key");
        let value = validate_required_string!(args, "value");

        let mut cache = self.cache.lock().await;
        cache.insert(key.to_string(), value.to_string());

        tool_result!(success: format!("Cached: {} = {}", key, value))
    }
}
```

### Example 3: Complex Validation

```rust
use llm_fusion::{tool_metadata, validate_required_string, validate_required_number, tool_result};

pub struct MathTool;

#[async_trait]
impl Tool for MathTool {
    fn metadata(&self) -> ToolMetadata {
        tool_metadata! {
            name: "divide",
            description: "Divide two numbers",
            parameters: [
                { name: "a", type: "number", description: "Dividend", required: true },
                { name: "b", type: "number", description: "Divisor", required: true }
            ]
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let a = validate_required_number!(args, "a");
        let b = validate_required_number!(args, "b");

        if b == 0 {
            return Err(anyhow::anyhow!("Cannot divide by zero"));
        }

        if a > i64::MAX / 2 || a < i64::MIN / 2 {
            return Err(anyhow::anyhow!("Dividend too large"));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let a = validate_required_number!(args, "a");
        let b = validate_required_number!(args, "b");

        let result = a / b;
        tool_result!(success: format!("{} / {} = {}", a, b, result))
    }
}
```

### Example 4: Async External Call

```rust
use llm_fusion::{tool_metadata, validate_required_string, validate_optional_string, tool_result};

pub struct HttpTool {
    client: reqwest::Client,
    timeout_secs: u64,
}

impl HttpTool {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            client: reqwest::Client::new(),
            timeout_secs,
        }
    }
}

#[async_trait]
impl Tool for HttpTool {
    fn metadata(&self) -> ToolMetadata {
        tool_metadata! {
            name: "http_request",
            description: "Make an HTTP request",
            parameters: [
                { name: "url", type: "string", description: "URL to request", required: true },
                { name: "method", type: "string", description: "HTTP method (default: GET)", required: false }
            ]
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let url = validate_required_string!(args, "url");

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow::anyhow!("URL must start with http:// or https://"));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let url = validate_required_string!(args, "url");
        let method = validate_optional_string!(args, "method", "GET");

        let request = match method.to_uppercase().as_str() {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            _ => return tool_result!(failure: format!("Unsupported method: {}", method)),
        };

        match request.timeout(Duration::from_secs(self.timeout_secs)).send().await {
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                tool_result!(success: format!("Status: {}\n\n{}", status, body))
            }
            Err(e) => tool_result!(failure: format!("Request failed: {}", e)),
        }
    }
}
```

---

## Summary

### Key Takeaways

1. **Tool Trait** is the core abstraction - metadata, validate, execute
2. **Three approaches** to create tools - manual, declarative macros, proc macro
3. **ToolRegistry** manages discovery and execution
4. **Validation** happens in two layers - parameters and business logic
5. **Macros** reduce boilerplate by 41-50%

### Quick Reference

| Task | Code |
|------|------|
| Define metadata | `tool_metadata! { ... }` or `#[tool(...)]` |
| Validate string | `validate_required_string!(args, "name")` |
| Validate number | `validate_required_number!(args, "count")` |
| Optional param | `validate_optional_string!(args, "opt", "default")` |
| Return success | `tool_result!(success: "done")` |
| Return failure | `tool_result!(failure: "error")` |

### File Locations

- **Core trait**: `src/tools/mod.rs`
- **Declarative macros**: `src/tools/macros.rs`
- **Procedural macro**: `llm_fusion_macros/src/lib.rs`
- **Built-in tools**: `src/tools/{filesystem,shell,http}.rs`
- **Registry**: `src/tools/registry.rs`
- **Examples**: `examples/tool_*.rs`

### Next Steps

1. Read the examples: `examples/tool_with_macro.rs`, `examples/tool_proc_macro.rs`
2. Run the examples: `cargo run --example tool_with_macro`
3. Create your first tool using the proc macro approach
4. Graduate to declarative macros for complex cases
5. Contribute new tools to the registry

---

**For more information:**
- `TOOL_USAGE.md` - Step-by-step guide to adding tools
- `TOOL_MACROS.md` - Detailed macro documentation
- `examples/` - Working examples of all patterns
