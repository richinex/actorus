use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{timeout, Duration};
use std::sync::OnceLock;
use crate::actors::messages::*;
use crate::config::Settings;
use crate::core::mcp::MCPClient;

static ROUTER_SENDER: OnceLock<Sender<RoutingMessage>> = OnceLock::new();

pub struct MCPActorHandle {
    sender: Sender<MCPMessage>,
}

impl MCPActorHandle {
    pub fn new(settings: Settings) -> Self {
        let buffer_size = settings.system.channel_buffer_size;
        let (sender, receiver) = channel(buffer_size);
        tokio::spawn(mcp_actor(receiver, settings));
        Self { sender }
    }

    pub async fn send_message(&self, message: MCPMessage) -> anyhow::Result<()> {
        self.sender
            .send(message)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send message to MCP actor: {}", e))
    }
}

async fn mcp_actor(mut receiver: Receiver<MCPMessage>, settings: Settings) {
    let timeout_duration = Duration::from_millis(settings.system.check_interval_ms);

    tracing::info!("MCP actor started");

    loop {
        match timeout(timeout_duration, receiver.recv()).await {
            Ok(Some(message)) => {
                handle_mcp_message(message).await;
            }
            Ok(None) => {
                tracing::info!("MCP actor channel closed, shutting down");
                break;
            }
            Err(_) => {
                send_heartbeat();
            }
        }
    }
}

async fn handle_mcp_message(message: MCPMessage) {
    match message {
        MCPMessage::ListTools(request) => {
            let args_refs: Vec<&str> = request.server_args.iter().map(|s| s.as_str()).collect();

            match MCPClient::new(&request.server_command, args_refs).await {
                Ok(mut client) => match client.list_tools().await {
                    Ok(tools) => {
                        let tool_names: Vec<String> =
                            tools.iter().map(|t| t.name.clone()).collect();
                        let _ = request.response.send(MCPResponse::Tools(tool_names));
                    }
                    Err(e) => {
                        tracing::error!("Failed to list tools: {}", e);
                        let _ = request.response.send(MCPResponse::Error(e.to_string()));
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to create MCP client: {}", e);
                    let _ = request.response.send(MCPResponse::Error(e.to_string()));
                }
            }
        }
        MCPMessage::CallTool(request) => {
            let args_refs: Vec<&str> = request.server_args.iter().map(|s| s.as_str()).collect();

            match MCPClient::new(&request.server_command, args_refs).await {
                Ok(mut client) => {
                    match client.call_tool(&request.tool_name, request.arguments).await {
                        Ok(result) => {
                            let _ = request.response.send(MCPResponse::Content(result));
                        }
                        Err(e) => {
                            tracing::error!("Failed to call tool: {}", e);
                            let _ = request.response.send(MCPResponse::Error(e.to_string()));
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create MCP client: {}", e);
                    let _ = request.response.send(MCPResponse::Error(e.to_string()));
                }
            }
        }
    }
}

fn send_heartbeat() {
    if let Some(router) = ROUTER_SENDER.get() {
        let _ = router.try_send(RoutingMessage::Heartbeat(ActorType::MCP));
    }
}

pub fn set_router_sender(sender: Sender<RoutingMessage>) {
    let _ = ROUTER_SENDER.set(sender);
}
