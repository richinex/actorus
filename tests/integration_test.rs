//! Integration tests for LLM Fusion
//!
//! These tests verify the system works without requiring API keys

use actorus::tools::{
    executor::ToolExecutor,
    filesystem::{ReadFileTool, WriteFileTool},
    registry::ToolRegistry,
    shell::ShellTool,
    http::HttpTool,
    Tool,
    ToolConfig,
};
use serde_json::json;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_tool_registry_initialization() {
    let registry = ToolRegistry::with_defaults();

    // Verify all default tools are registered
    assert!(registry.has_tool("execute_shell"));
    assert!(registry.has_tool("read_file"));
    assert!(registry.has_tool("write_file"));
    assert!(registry.has_tool("http_request"));

    let tools = registry.list_tools();
    assert_eq!(tools.len(), 4);
}

#[tokio::test]
async fn test_tool_registry_description() {
    let registry = ToolRegistry::with_defaults();
    let description = registry.tools_description();

    // Verify description contains key information
    assert!(description.contains("execute_shell"));
    assert!(description.contains("Description:"));
    assert!(description.contains("Parameters:"));
}

#[tokio::test]
async fn test_shell_tool_execution() {
    let tool = ShellTool::new(5);
    let args = json!({"command": "echo 'Hello from shell'"});

    let result = tool.execute(args).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Hello from shell"));
}

#[tokio::test]
async fn test_filesystem_write_and_read() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");

    // Write
    let write_tool = WriteFileTool::new(1024 * 1024);
    let write_args = json!({
        "path": file_path.to_str().unwrap(),
        "content": "Test content from integration test"
    });

    let write_result = write_tool.execute(write_args).await.unwrap();
    assert!(write_result.success);

    // Read
    let read_tool = ReadFileTool::new(1024 * 1024);
    let read_args = json!({
        "path": file_path.to_str().unwrap()
    });

    let read_result = read_tool.execute(read_args).await.unwrap();
    assert!(read_result.success);
    assert_eq!(read_result.output, "Test content from integration test");
}

#[tokio::test]
async fn test_tool_executor_retry() {
    let executor = ToolExecutor::new(ToolConfig {
        timeout_secs: 30,
        max_retries: 3,
        sandbox: false,
    });

    let tool = Arc::new(ShellTool::new(5));
    let args = json!({"command": "echo 'retry test'"});

    let result = executor.execute(tool, args).await.unwrap();
    assert!(result.success);
}

#[tokio::test]
async fn test_shell_tool_whitelist() {
    let tool = ShellTool::new(5).with_whitelist(vec![
        "echo".to_string(),
        "ls".to_string(),
    ]);

    // Allowed command
    let args = json!({"command": "echo 'allowed'"});
    assert!(tool.validate(&args).is_ok());

    // Disallowed command
    let args = json!({"command": "rm -rf /"});
    assert!(tool.validate(&args).is_err());
}

#[tokio::test]
async fn test_filesystem_size_limits() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("large.txt");

    // Create a large file
    let large_content = "x".repeat(100);
    std::fs::write(&file_path, &large_content).unwrap();

    // Try to read with small size limit
    let tool = ReadFileTool::new(50); // 50 bytes max
    let args = json!({
        "path": file_path.to_str().unwrap()
    });

    let result = tool.execute(args).await.unwrap();
    assert!(!result.success); // Should fail due to size limit
    assert!(result.error.unwrap().contains("too large"));
}

#[tokio::test]
async fn test_http_tool_validation() {
    let tool = HttpTool::new(10).with_allowed_domains(vec![
        "example.com".to_string(),
    ]);

    // Allowed domain
    let args = json!({"url": "https://example.com/api"});
    assert!(tool.validate(&args).is_ok());

    // Disallowed domain
    let args = json!({"url": "https://malicious.com/steal"});
    assert!(tool.validate(&args).is_err());
}

#[tokio::test]
async fn test_tool_metadata() {
    let shell_tool = ShellTool::new(5);
    let metadata = shell_tool.metadata();

    assert_eq!(metadata.name, "execute_shell");
    assert!(!metadata.description.is_empty());
    assert!(!metadata.parameters.is_empty());

    // Check parameter structure
    let param = &metadata.parameters[0];
    assert_eq!(param.name, "command");
    assert_eq!(param.param_type, "string");
    assert!(param.required);
}

#[tokio::test]
async fn test_tool_executor_backoff() {
    use std::time::Instant;

    let executor = ToolExecutor::new(ToolConfig {
        timeout_secs: 5,
        max_retries: 3,
        sandbox: false,
    });

    // This will fail and should retry with backoff
    let tool = Arc::new(ShellTool::new(1));
    let args = json!({"command": "sleep 10"}); // Will timeout

    let start = Instant::now();
    let result = executor.execute(tool, args).await.unwrap();
    let duration = start.elapsed();

    assert!(!result.success);
    // With retries and backoff, should take longer than just one timeout
    assert!(duration.as_secs() >= 3); // At least 3 seconds for retries
}
