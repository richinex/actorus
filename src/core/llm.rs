use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use futures::StreamExt;
use crate::config::Settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema {
        json_schema: JsonSchemaFormat,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaFormat {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub schema: Value,
    #[serde(default = "default_strict")]
    pub strict: bool,
}

fn default_strict() -> bool {
    true
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Delta,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

pub struct LLMClient {
    client: Client,
    api_key: String,
    settings: Settings,
}

impl LLMClient {
    pub fn new(api_key: String, settings: Settings) -> Self {
        Self {
            client: Client::new(),
            api_key,
            settings,
        }
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        self.chat_with_format(messages, None).await
    }

    pub async fn chat_with_format(
        &self,
        messages: Vec<ChatMessage>,
        response_format: Option<ResponseFormat>,
    ) -> Result<String> {
        let request = ChatRequest {
            model: self.settings.llm.model.clone(),
            messages,
            max_tokens: self.settings.llm.max_tokens,
            temperature: self.settings.llm.temperature,
            stream: false,
            response_format,
        };

        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 1000;

        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = BASE_DELAY_MS * 2_u64.pow(attempt - 1);
                tracing::warn!(
                    "[LLMClient] Retrying API call (attempt {}/{}) after {}ms delay",
                    attempt + 1,
                    MAX_RETRIES,
                    delay
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }

            let response_result = self
                .client
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await;

            let response = match response_result {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::warn!("[LLMClient] HTTP request failed: {}", e);
                    last_error = Some(anyhow::anyhow!("HTTP request failed: {}", e));
                    continue;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                tracing::warn!(
                    "[LLMClient] API returned error status {}: {}",
                    status,
                    error_text
                );
                last_error = Some(anyhow::anyhow!("API error {}: {}", status, error_text));
                continue;
            }

            let chat_response = match response.json::<ChatResponse>().await {
                Ok(cr) => cr,
                Err(e) => {
                    tracing::warn!("[LLMClient] Failed to decode response body: {}", e);
                    last_error = Some(anyhow::anyhow!("Response decode error: {}", e));
                    continue;
                }
            };

            return Ok(chat_response
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_default());
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
    }

    pub async fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
        tx: mpsc::Sender<String>,
    ) -> Result<()> {
        let request = ChatRequest {
            model: self.settings.llm.model.clone(),
            messages,
            max_tokens: self.settings.llm.max_tokens,
            temperature: self.settings.llm.temperature,
            stream: true,
            response_format: None,
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            if let Ok(bytes) = chunk {
                let text = String::from_utf8_lossy(&bytes);

                for line in text.lines() {
                    if line.starts_with("data: ") {
                        let json_str = &line[6..];
                        if json_str == "[DONE]" {
                            break;
                        }

                        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) {
                            if let Some(content) = chunk
                                .choices
                                .first()
                                .and_then(|c| c.delta.content.as_ref())
                            {
                                tx.send(content.clone()).await?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
