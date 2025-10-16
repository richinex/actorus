use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::tools::{Tool, ToolMetadata, ToolParameter, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_input_schema")]
    pub input_schema: serde_json::Value,
}

fn default_input_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {}
    })
}

#[derive(Debug, Serialize, Deserialize)]
struct MCPResponse {
    jsonrpc: String,
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<MCPError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MCPError {
    code: i32,
    message: String,
}

pub struct MCPClient {
    process: Child,
    request_id: u64,
}

impl MCPClient {
    pub async fn new(command: &str, args: Vec<&str>) -> Result<Self> {
        let process = Command::new(command)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let mut client = Self {
            process,
            request_id: 0,
        };

        client.initialize().await?;
        Ok(client)
    }

    async fn initialize(&mut self) -> Result<()> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "llm-fusion",
                    "version": "0.1.0"
                }
            }
        });

        self.send_request(&request).await?;
        let _response = self.read_response().await?;
        Ok(())
    }

    pub async fn list_tools(&mut self) -> Result<Vec<MCPTool>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "tools/list"
        });

        self.send_request(&request).await?;
        let response = self.read_response().await?;

        if let Some(result) = response.result {
            let tools: Vec<MCPTool> =
                serde_json::from_value(result.get("tools").unwrap_or(&json!([])).clone())?;
            Ok(tools)
        } else {
            Ok(vec![])
        }
    }

    pub async fn call_tool(&mut self, name: &str, arguments: serde_json::Value) -> Result<String> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments
            }
        });

        self.send_request(&request).await?;
        let response = self.read_response().await?;

        if let Some(result) = response.result {
            Ok(serde_json::to_string_pretty(&result)?)
        } else if let Some(error) = response.error {
            Err(anyhow::anyhow!("Tool call failed: {}", error.message))
        } else {
            Err(anyhow::anyhow!("No result from tool call"))
        }
    }

    async fn send_request(&mut self, request: &serde_json::Value) -> Result<()> {
        let stdin = self
            .process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?;

        let json = serde_json::to_string(request)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        Ok(())
    }

    async fn read_response(&mut self) -> Result<MCPResponse> {
        let stdout = self
            .process
            .stdout
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: MCPResponse = serde_json::from_str(&line)?;
        Ok(response)
    }

    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }
}

impl Drop for MCPClient {
    fn drop(&mut self) {
        let _ = self.process.start_kill();
    }
}

// ============================================================================
// MCP Tool Wrapper - Makes ANY MCP tool usable in agent system
// ============================================================================

/// Wraps an MCP tool to make it usable in the agent system.
/// This wrapper handles the conversion between agent Tool trait and MCP server calls.
pub struct MCPToolWrapper {
    tool_name: String,
    description: String,
    input_schema: serde_json::Value,
    server_command: String,
    server_args: Vec<String>,
}

#[async_trait]
impl Tool for MCPToolWrapper {
    fn metadata(&self) -> ToolMetadata {
        // Extract parameters from JSON schema
        let parameters = if let Some(props) = self.input_schema.get("properties") {
            if let Some(obj) = props.as_object() {
                obj.iter()
                    .map(|(name, schema)| {
                        let description = schema
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string();

                        let param_type = schema
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("string")
                            .to_string();

                        let required = self
                            .input_schema
                            .get("required")
                            .and_then(|r| r.as_array())
                            .map(|arr| arr.iter().any(|v| v.as_str() == Some(name)))
                            .unwrap_or(false);

                        ToolParameter {
                            name: name.clone(),
                            description,
                            param_type,
                            required,
                        }
                    })
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        ToolMetadata {
            name: self.tool_name.clone(),
            description: self.description.clone(),
            parameters,
        }
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        // Create a new MCP client for each execution
        let args_refs: Vec<&str> = self.server_args.iter().map(|s| s.as_str()).collect();
        let mut client = MCPClient::new(&self.server_command, args_refs).await?;

        // Call the tool
        let result = client.call_tool(&self.tool_name, args).await?;

        Ok(ToolResult::success(result))
    }
}

// ============================================================================
// MCP Tool Discovery
// ============================================================================

/// Discover all tools from an MCP server and create tool wrappers.
///
/// This is the main entry point for integrating MCP servers into your agent system.
/// Simply provide the server command and args, and get back ready-to-use tools.
///
/// # Example
/// ```no_run
/// use actorus::core::mcp::discover_mcp_tools;
///
/// let tools = discover_mcp_tools(
///     "npx",
///     vec!["-y", "@modelcontextprotocol/server-brave-search"]
/// ).await?;
///
/// // Add tools to agent
/// let agent = AgentBuilder::new("research_agent")
///     .tools(tools);
/// ```
pub async fn discover_mcp_tools(
    server_command: &str,
    server_args: Vec<&str>,
) -> Result<Vec<Arc<dyn Tool>>> {
    tracing::info!(
        "Discovering tools from MCP server: {} {}",
        server_command,
        server_args.join(" ")
    );

    let mut client = MCPClient::new(server_command, server_args.clone()).await?;
    let tools = client.list_tools().await?;

    tracing::info!("Found {} tools from MCP server", tools.len());

    let mut tool_wrappers: Vec<Arc<dyn Tool>> = Vec::new();

    for mcp_tool in tools {
        tracing::debug!("Wrapping MCP tool: {}", mcp_tool.name);

        let wrapper = MCPToolWrapper {
            tool_name: mcp_tool.name.clone(),
            description: mcp_tool.description.clone().unwrap_or_default(),
            input_schema: mcp_tool.input_schema.clone(),
            server_command: server_command.to_string(),
            server_args: server_args.iter().map(|s| s.to_string()).collect(),
        };

        tool_wrappers.push(Arc::new(wrapper));
    }

    Ok(tool_wrappers)
}
