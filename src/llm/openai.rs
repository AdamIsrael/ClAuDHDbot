use reqwest::Client;
use serde_json::json;

use crate::config::LlmConfig;
use crate::mcp::ToolDefinition;

use super::{ChatMessage, LlmProvider, LlmResponse, MessageContent, Role, ToolCall};

/// Provider for any OpenAI-compatible API (OpenAI, vLLM, LMStudio, llama.cpp, etc.)
pub struct OpenAiProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
}

impl OpenAiProvider {
    pub fn new(config: &LlmConfig) -> anyhow::Result<Self> {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        Ok(Self {
            client: Client::new(),
            base_url,
            api_key: config.api_key.clone(),
            model: config.model.clone(),
        })
    }

    fn build_messages(&self, messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        let mut api_messages = Vec::new();

        for msg in messages {
            match (&msg.role, &msg.content) {
                (Role::System, MessageContent::Text(text)) => {
                    api_messages.push(json!({
                        "role": "system",
                        "content": text,
                    }));
                }
                (Role::User, MessageContent::Text(text)) => {
                    api_messages.push(json!({
                        "role": "user",
                        "content": text,
                    }));
                }
                (Role::Assistant, MessageContent::Text(text)) => {
                    api_messages.push(json!({
                        "role": "assistant",
                        "content": text,
                    }));
                }
                (Role::Assistant, MessageContent::ToolUse { id, name, input }) => {
                    api_messages.push(json!({
                        "role": "assistant",
                        "tool_calls": [{
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": input.to_string(),
                            },
                        }],
                    }));
                }
                (
                    Role::User,
                    MessageContent::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    },
                ) => {
                    api_messages.push(json!({
                        "role": "tool",
                        "tool_call_id": tool_use_id,
                        "content": content,
                    }));
                }
                _ => {}
            }
        }

        api_messages
    }

    fn build_tools(&self, tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.qualified_name(),
                        "description": t.description,
                        "parameters": t.input_schema,
                    },
                })
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> anyhow::Result<LlmResponse> {
        let api_messages = self.build_messages(messages);
        let api_tools = self.build_tools(tools);

        let mut body = json!({
            "model": self.model,
            "messages": api_messages,
        });

        if !api_tools.is_empty() {
            body["tools"] = json!(api_tools);
        }

        let mut request = self.client.post(format!(
            "{}/chat/completions",
            self.base_url.trim_end_matches('/')
        ));

        if let Some(ref api_key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {api_key}"));
        }

        let response = request
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let response_body: serde_json::Value = response.json().await?;

        if !status.is_success() {
            let error_msg = response_body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown API error");
            return Err(anyhow::anyhow!("OpenAI API error ({status}): {error_msg}"));
        }

        let choice = &response_body["choices"][0];
        let message = &choice["message"];

        // Check for tool calls
        if let Some(tool_calls_arr) = message["tool_calls"].as_array() {
            let tool_calls: Vec<ToolCall> = tool_calls_arr
                .iter()
                .filter_map(|tc| {
                    let id = tc["id"].as_str()?.to_string();
                    let name = tc["function"]["name"].as_str()?.to_string();
                    let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                    let input = serde_json::from_str(args_str).unwrap_or(json!({}));
                    Some(ToolCall { id, name, input })
                })
                .collect();

            if !tool_calls.is_empty() {
                return Ok(LlmResponse::ToolUse(tool_calls));
            }
        }

        let text = message["content"].as_str().unwrap_or("").to_string();

        Ok(LlmResponse::Text(text))
    }
}
