use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{timeout, Duration};
use std::sync::OnceLock;
use tokio::sync::oneshot;
use crate::actors::messages::*;
use crate::config::Settings;
use crate::core::llm::LLMClient;


static ROUTER_SENDER: OnceLock<Sender<RoutingMessage>> = OnceLock::new();

pub struct LLMActorHandle {
    sender: Sender<LLMMessage>,
}

impl LLMActorHandle {
    pub fn new(settings: Settings, api_key: String) -> Self {
        let buffer_size = settings.system.channel_buffer_size;
        let (sender, receiver) = channel(buffer_size);
        tokio::spawn(llm_actor(receiver, settings, api_key));
        Self { sender }
    }

    pub async fn send_message(&self, message: LLMMessage) -> anyhow::Result<()> {
        self.sender
            .send(message)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send message to LLM actor: {}", e))
    }
}

async fn llm_actor(
    mut receiver: Receiver<LLMMessage>,
    settings: Settings,
    api_key: String,
) {
    let client = LLMClient::new(api_key, settings.clone());
    let timeout_duration = Duration::from_millis(settings.system.check_interval_ms);

    tracing::info!("LLM actor started");

    loop {
        match timeout(timeout_duration, receiver.recv()).await {
            Ok(Some(message)) => {
                handle_llm_message(message, &client).await;
            }
            Ok(None) => {
                tracing::info!("LLM actor channel closed, shutting down");
                break;
            }
            Err(_) => {
                send_heartbeat();
            }
        }
    }
}

async fn handle_llm_message(message: LLMMessage, client: &LLMClient) {
    match message {
        LLMMessage::Chat(chat_request) => {
            let messages: Vec<_> = chat_request
                .messages
                .iter()
                .map(|m| crate::core::llm::ChatMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect();

            if chat_request.stream {
                handle_stream_chat(messages, client, chat_request.response).await;
            } else {
                handle_regular_chat(messages, client, chat_request.response).await;
            }
        }
    }
}

async fn handle_regular_chat(
    messages: Vec<crate::core::llm::ChatMessage>,
    client: &LLMClient,
    response_channel: oneshot::Sender<ChatResponse>,
) {
    match client.chat(messages).await {
        Ok(content) => {
            let _ = response_channel.send(ChatResponse::Complete(content));
        }
        Err(e) => {
            tracing::error!("LLM chat error: {}", e);
            let _ = response_channel.send(ChatResponse::Error(e.to_string()));
        }
    }
}

async fn handle_stream_chat(
    messages: Vec<crate::core::llm::ChatMessage>,
    client: &LLMClient,
    response_channel: oneshot::Sender<ChatResponse>,
) {
    let (tx, rx) = channel(100);

    // Send receiver back immediately
    let _ = response_channel.send(ChatResponse::StreamTokens(rx));

    // Start streaming
    if let Err(e) = client.stream_chat(messages, tx).await {
        tracing::error!("Stream error: {}", e);
    }
}

fn send_heartbeat() {
    if let Some(router) = ROUTER_SENDER.get() {
        let _ = router.try_send(RoutingMessage::Heartbeat(ActorType::LLM));
    }
}

pub fn set_router_sender(sender: Sender<RoutingMessage>) {
    let _ = ROUTER_SENDER.set(sender);
}
