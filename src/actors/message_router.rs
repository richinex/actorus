use crate::actors::llm_actor::LLMActorHandle;
use crate::actors::mcp_actor::MCPActorHandle;
use crate::actors::agent_actor::AgentActorHandle;
use crate::actors::messages::*;
use crate::actors::health_monitor::health_monitor_actor;
use crate::config::Settings;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{sleep, Duration};

pub struct MessageRouterHandle {
    sender: Sender<RoutingMessage>,
}

impl MessageRouterHandle {
    pub fn new(settings: Settings, api_key: String) -> Self {
        let buffer_size = settings.system.channel_buffer_size;
        let (sender, receiver) = channel(buffer_size);
        tokio::spawn(router_actor(receiver, settings, api_key));
        Self { sender }
    }

    pub async fn send_message(&self, message: RoutingMessage) -> anyhow::Result<()> {
        self.sender
            .send(message)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send message to Router: {}", e))
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.send_message(RoutingMessage::Shutdown).await
    }
}

async fn router_actor(mut receiver: Receiver<RoutingMessage>, settings: Settings, api_key: String) {
    tracing::info!("Router actor started");

    let mut llm_handle = LLMActorHandle::new(settings.clone(), api_key.clone());
    let mut mcp_handle = MCPActorHandle::new(settings.clone());
    let mut agent_handle = AgentActorHandle::new(settings.clone(), api_key.clone());

    // Create supervisor channel
    let (supervisor_sender, supervisor_receiver) = channel(settings.system.channel_buffer_size);

    // ✅ Create a sender clone that supervisor can use to send Reset messages back to router
    let (router_tx, mut router_rx) = channel(settings.system.channel_buffer_size);

    // Spawn health monitor with the router_tx so it can send Reset messages
    tokio::spawn(health_monitor_actor(
        supervisor_receiver,
        router_tx,
        settings.clone(),
    ));

    crate::actors::llm_actor::set_router_sender(supervisor_sender.clone());
    crate::actors::mcp_actor::set_router_sender(supervisor_sender.clone());
    crate::actors::agent_actor::set_router_sender(supervisor_sender.clone());

    // ✅ Add heartbeat interval for Router
    let heartbeat_interval = Duration::from_millis(settings.system.heartbeat_interval_ms);
    let mut heartbeat_timer = tokio::time::interval(heartbeat_interval);

    loop {
        tokio::select! {
            // Handle incoming messages from external API
            Some(message) = receiver.recv() => {
                match message {
                    RoutingMessage::LLM(llm_message) => {
                        if let Err(e) = llm_handle.send_message(llm_message).await {
                            tracing::error!("Failed to send to LLM actor: {}", e);
                        }
                    }
                    RoutingMessage::MCP(mcp_message) => {
                        if let Err(e) = mcp_handle.send_message(mcp_message).await {
                            tracing::error!("Failed to send to MCP actor: {}", e);
                        }
                    }
                    RoutingMessage::Agent(agent_message) => {
                        if let Err(e) = agent_handle.send_message(agent_message).await {
                            tracing::error!("Failed to send to Agent actor: {}", e);
                        }
                    }
                    // ✅ Handle GetState from external API
                    RoutingMessage::GetState(response_tx) => {
                        // Forward to supervisor
                        let _ = supervisor_sender
                            .send(RoutingMessage::GetState(response_tx))
                            .await;
                    }
                    RoutingMessage::Shutdown => {
                        tracing::info!("Router received shutdown signal from external");
                        let _ = supervisor_sender.send(RoutingMessage::Shutdown).await;
                        break;
                    }
                    _ => {
                        tracing::debug!("Router received unexpected message from external interface");
                    }
                }
            }

            // ✅ Handle internal messages (from supervisor, like Reset)
            Some(message) = router_rx.recv() => {
                match message {
                    RoutingMessage::Heartbeat(actor_type) => {
                        // Forward heartbeats to supervisor
                        let _ = supervisor_sender
                            .send(RoutingMessage::Heartbeat(actor_type))
                            .await;
                    }
                    RoutingMessage::Reset(actor_type) => {
                        if settings.system.auto_restart {
                            tracing::warn!("Resetting actor: {:?}", actor_type);
                            match actor_type {
                                ActorType::LLM => {
                                    llm_handle = LLMActorHandle::new(settings.clone(), api_key.clone());
                                    sleep(Duration::from_millis(100)).await;
                                    tracing::info!("LLM actor reset complete");
                                }
                                ActorType::MCP => {
                                    mcp_handle = MCPActorHandle::new(settings.clone());
                                    sleep(Duration::from_millis(100)).await;
                                    tracing::info!("MCP actor reset complete");
                                }
                                ActorType::Agent => {
                                    agent_handle = AgentActorHandle::new(settings.clone(), api_key.clone());
                                    sleep(Duration::from_millis(100)).await;
                                    tracing::info!("Agent actor reset complete");
                                }
                                ActorType::Router => {
                                    tracing::warn!("Cannot reset Router from within itself");
                                }
                                ActorType::Supervisor => {
                                    tracing::warn!("Cannot reset Supervisor");
                                }
                            }
                        } else {
                            tracing::warn!("Auto-restart disabled, ignoring reset for {:?}", actor_type);
                        }
                    }
                    RoutingMessage::Shutdown => {
                        tracing::info!("Router received shutdown signal from supervisor");
                        let _ = supervisor_sender.send(RoutingMessage::Shutdown).await;
                        break;
                    }
                    _ => {}
                }
            }

            // ✅ Send Router's own heartbeat periodically
            _ = heartbeat_timer.tick() => {
                let _ = supervisor_sender
                    .send(RoutingMessage::Heartbeat(ActorType::Router))
                    .await;
                tracing::trace!("Router sent heartbeat");
            }
        }
    }
}
