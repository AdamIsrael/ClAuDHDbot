use reqwest::Client;
use serde_json::json;

use crate::config::LlmConfig;
use crate::mcp::ToolDefinition;

use super::{ChatMessage, LlmProvider, LlmResponse, MessageContent, Role};

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(config: &LlmConfig) -> anyhow::Result<Self> {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        Ok(Self {
            client: Client::new(),
            base_url,
            model: config.model.clone(),
        })
    }
}

#[async_trait::async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        _tools: &[ToolDefinition],
    ) -> anyhow::Result<LlmResponse> {
        // Ollama uses a simpler message format
        let api_messages: Vec<serde_json::Value> = messages
            .iter()
            .filter_map(|msg| {
                let role = match msg.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                };
                match &msg.content {
                    MessageContent::Text(text) => Some(json!({
                        "role": role,
                        "content": text,
                    })),
                    _ => None,
                }
            })
            .collect();

        let body = json!({
            "model": self.model,
            "messages": api_messages,
            "stream": false,
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let response_body: serde_json::Value = response.json().await?;

        if !status.is_success() {
            let error_msg = response_body["error"]
                .as_str()
                .unwrap_or("Unknown Ollama error");
            return Err(anyhow::anyhow!("Ollama API error ({status}): {error_msg}"));
        }

        let text = response_body["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(LlmResponse::Text(text))
    }
}
