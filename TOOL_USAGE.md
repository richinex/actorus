# Tool System Usage Guide

## Overview

The tool system in LLM Fusion allows agents to interact with the external world through a well-defined interface. Tools are modular, testable, and easy to extend.

## Architecture

```
Tool Trait (interface)
    ↓
Concrete Tool Implementations (ShellTool, ReadFileTool, etc.)
    ↓
ToolRegistry (manages available tools)
    ↓
ToolExecutor (executes tools with retry/timeout)
    ↓
Agents (use tools to accomplish tasks)
```

## Available Tools

### File System Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `read_file` | Read file contents | `path` (string) |
| `write_file` | Write content to file | `path` (string), `content` (string) |
| `append_file` | Append content to file | `path` (string), `content` (string) |

### Shell Tool

| Tool | Description | Parameters |
|------|-------------|------------|
| `execute_shell` | Execute shell command | `command` (string) |

### HTTP Tool

| Tool | Description | Parameters |
|------|-------------|------------|
| `http_request` | Make HTTP request | `url` (string), `method` (string, optional) |

## Creating a New Tool

### Step 1: Implement the Tool

Create your tool in the appropriate module (e.g., `src/tools/your_tool.rs`):

```rust
use super::{Tool, ToolMetadata, ToolParameter, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Your custom tool
pub struct YourTool {
    // Configuration fields
    max_size: usize,
    timeout: u64,
}

impl YourTool {
    pub fn new(timeout: u64) -> Self {
        Self {
            max_size: 1024 * 1024, // 1MB
            timeout,
        }
    }
}

#[async_trait]
impl Tool for YourTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "your_tool".to_string(),
            description: "Description of what your tool does".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "param1".to_string(),
                    param_type: "string".to_string(),
                    description: "Description of param1".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "param2".to_string(),
                    param_type: "number".to_string(),
                    description: "Description of param2".to_string(),
                    required: false,
                },
            ],
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        // Validate required parameters
        let param1 = args["param1"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'param1' is required"))?;

        if param1.is_empty() {
            return Err(anyhow::anyhow!("param1 cannot be empty"));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let param1 = args["param1"].as_str().unwrap();

        tracing::info!("Executing your_tool with param1: {}", param1);

        // Your tool logic here
        match perform_operation(param1).await {
            Ok(result) => Ok(ToolResult::success(result)),
            Err(e) => Ok(ToolResult::failure(format!("Operation failed: {}", e))),
        }
    }
}
```

### Step 2: Register the Tool

Add your tool to `src/tools/registry.rs` in the `with_defaults()` method:

```rust
pub fn with_defaults() -> Self {
    let mut registry = Self::new();

    // Existing tools
    registry.register(Arc::new(crate::tools::shell::ShellTool::new(30)));
    registry.register(Arc::new(crate::tools::filesystem::ReadFileTool::new(1024 * 1024)));
    registry.register(Arc::new(crate::tools::filesystem::WriteFileTool::new(1024 * 1024)));
    registry.register(Arc::new(crate::tools::filesystem::AppendFileTool::new(1024 * 1024)));
    registry.register(Arc::new(crate::tools::http::HttpTool::new(30)));

    // Add your new tool
    registry.register(Arc::new(crate::tools::your_tool::YourTool::new(60)));

    registry
}
```

### Step 3: Add Tests

Add tests in your tool module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_your_tool_success() {
        let tool = YourTool::new(30);
        let args = json!({
            "param1": "test_value"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_your_tool_validation() {
        let tool = YourTool::new(30);
        let args = json!({
            "param1": ""
        });

        let result = tool.execute(args).await.unwrap();
        assert!(!result.success);
    }
}
```

### Step 4: Export the Tool

If you created a new file, export it in `src/tools/mod.rs`:

```rust
pub mod executor;
pub mod registry;
pub mod shell;
pub mod filesystem;
pub mod http;
pub mod your_tool;  // Add this line
```

### Step 5: Build and Test

```bash
# Run tests
cargo test your_tool

# Build the project
cargo build

# Test with an agent
cargo run --example simple_agent_test
```

## Real-World Example: AppendFileTool

Here's how the `append_file` tool was implemented:

### Implementation (`src/tools/filesystem.rs`)

```rust
pub struct AppendFileTool {
    allowed_paths: Option<Vec<PathBuf>>,
    max_size_bytes: usize,
}

impl AppendFileTool {
    pub fn new(max_size_bytes: usize) -> Self {
        Self {
            allowed_paths: None,
            max_size_bytes,
        }
    }

    pub fn with_allowed_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.allowed_paths = Some(paths);
        self
    }
}

#[async_trait]
impl Tool for AppendFileTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "append_file".to_string(),
            description: "Append content to an existing file. Creates file if it doesn't exist.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "The file path to append to".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "content".to_string(),
                    param_type: "string".to_string(),
                    description: "The content to append".to_string(),
                    required: true,
                },
            ],
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let path_str = args["path"].as_str().unwrap();
        let content = args["content"].as_str().unwrap();
        let path = Path::new(path_str);

        tracing::info!("Appending to file: {}", path_str);

        use tokio::io::AsyncWriteExt;
        let result = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await;

        match result {
            Ok(mut file) => {
                match file.write_all(content.as_bytes()).await {
                    Ok(_) => Ok(ToolResult::success(format!(
                        "Successfully appended {} bytes to {}",
                        content.len(), path_str
                    ))),
                    Err(e) => Ok(ToolResult::failure(format!("Failed to write: {}", e))),
                }
            }
            Err(e) => Ok(ToolResult::failure(format!("Failed to open file: {}", e))),
        }
    }
}
```

### Registration

```rust
// In src/tools/registry.rs
registry.register(Arc::new(crate::tools::filesystem::AppendFileTool::new(1024 * 1024)));
```

### Tests

```rust
#[tokio::test]
async fn test_append_file_success() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("append_test.txt");

    // First write
    let write_tool = WriteFileTool::new(1024 * 1024);
    write_tool.execute(json!({
        "path": file_path.to_str().unwrap(),
        "content": "First line\n"
    })).await.unwrap();

    // Append
    let append_tool = AppendFileTool::new(1024 * 1024);
    let result = append_tool.execute(json!({
        "path": file_path.to_str().unwrap(),
        "content": "Second line\n"
    })).await.unwrap();

    assert!(result.success);

    let contents = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(contents, "First line\nSecond line\n");
}
```

## Best Practices

### 1. Information Hiding

Tools should hide implementation details:
- Path validation logic
- Error handling strategies
- Resource management (files, connections)
- Data format conversions

### 2. Security

Always implement security checks:
```rust
fn is_path_allowed(&self, path: &Path) -> bool {
    if let Some(ref allowed) = self.allowed_paths {
        allowed.iter().any(|p| path.starts_with(p))
    } else {
        true // Default: allow all (configure in production)
    }
}
```

### 3. Validation

Validate inputs before execution:
```rust
fn validate(&self, args: &Value) -> Result<()> {
    // Check required parameters
    let path = args["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("'path' is required"))?;

    // Validate values
    if path.is_empty() {
        return Err(anyhow::anyhow!("Path cannot be empty"));
    }

    // Check size limits
    if content.len() > self.max_size_bytes {
        return Err(anyhow::anyhow!("Content too large"));
    }

    Ok(())
}
```

### 4. Error Handling

Return `ToolResult` for execution errors (don't panic):
```rust
match operation().await {
    Ok(result) => Ok(ToolResult::success(result)),
    Err(e) => Ok(ToolResult::failure(format!("Operation failed: {}", e))),
}
```

### 5. Logging

Add informative logs:
```rust
tracing::info!("Executing operation: {}", operation_name);
tracing::debug!("Parameters: {:?}", params);
tracing::warn!("Retrying operation (attempt {})", retry_count);
tracing::error!("Operation failed: {}", error);
```

### 6. Testing

Test multiple scenarios:
- ✅ Successful execution
- ✅ Missing required parameters
- ✅ Invalid parameter values
- ✅ Size/resource limits
- ✅ Edge cases (empty values, special characters)
- ✅ Error conditions

## Tool Configuration

Tools can be configured when creating the registry:

```rust
// Custom configuration
let mut registry = ToolRegistry::new();

// Configure with custom limits
registry.register(Arc::new(
    ReadFileTool::new(5 * 1024 * 1024)  // 5MB max
        .with_allowed_paths(vec![
            PathBuf::from("./data"),
            PathBuf::from("./config"),
        ])
));

registry.register(Arc::new(
    ShellTool::new(60)  // 60 second timeout
        .with_whitelist(vec![
            "ls".to_string(),
            "find".to_string(),
            "grep".to_string(),
        ])
));
```

## Using Tools in Agents

Agents automatically have access to all registered tools. The LLM decides which tools to use based on the task:

```rust
// Agent will automatically choose the right tool
let result = agent::run_task(
    "Read the config.toml file and append a new line to it"
).await?;

// Agent will use: read_file → append_file
```

## Debugging Tools

Enable detailed logging:

```bash
# See tool execution
RUST_LOG=info cargo run --example simple_agent_test

# See tool parameters and validation
RUST_LOG=debug cargo run --example simple_agent_test

# See everything
RUST_LOG=trace cargo run --example simple_agent_test
```

## Tool Ideas

Here are some tools you might want to add:

### Data Processing
- `json_parse` - Parse and query JSON data
- `csv_read` - Read CSV files
- `xml_parse` - Parse XML documents

### Database
- `db_query` - Execute SQL queries
- `redis_get` - Get values from Redis
- `mongo_find` - Query MongoDB

### AI/ML
- `image_analyze` - Analyze images with vision models
- `text_summarize` - Summarize long text
- `embedding_generate` - Generate text embeddings

### System
- `process_list` - List running processes
- `environment_get` - Get environment variables
- `file_watch` - Watch files for changes

### Communication
- `email_send` - Send emails
- `slack_post` - Post to Slack
- `webhook_call` - Call webhooks

## Troubleshooting

### Tool not found
- Check it's registered in `with_defaults()`
- Verify the tool name matches exactly
- Check logs for registration messages

### Validation errors
- Ensure required parameters are provided
- Check parameter types match (string, number, etc.)
- Verify values meet constraints (size limits, etc.)

### Execution timeouts
- Increase timeout in ToolConfig
- Check for blocking operations
- Use async operations properly

### Permission denied
- Check `allowed_paths` configuration
- Verify file permissions
- Review security constraints

## Contributing

When adding tools to the project:

1. Follow the information hiding principle
2. Add comprehensive tests
3. Document parameters clearly
4. Implement proper validation
5. Add security checks
6. Include error handling
7. Add logging
8. Update this documentation

## References

- Tool Trait: `src/tools/mod.rs`
- Tool Registry: `src/tools/registry.rs`
- Tool Executor: `src/tools/executor.rs`
- Examples: `src/tools/filesystem.rs`, `src/tools/shell.rs`
- Tests: `src/tools/*/tests.rs`
