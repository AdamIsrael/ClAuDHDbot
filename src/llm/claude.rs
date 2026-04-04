use reqwest::Client;
use serde_json::json;

use crate::config::LlmConfig;
use crate::mcp::ToolDefinition;

use super::{ChatMessage, LlmProvider, LlmResponse, MessageContent, Role, ToolCall};

pub struct ClaudeProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl ClaudeProvider {
    pub fn new(config: &LlmConfig) -> anyhow::Result<Self> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Claude provider requires llm.api_key"))?;

        Ok(Self {
            client: Client::new(),
            api_key,
            model: config.model.clone(),
        })
    }

    fn build_messages(&self, messages: &[ChatMessage]) -> (Option<String>, Vec<serde_json::Value>) {
        let mut system = None;
        let mut api_messages = Vec::new();

        for msg in messages {
            match (&msg.role, &msg.content) {
                (Role::System, MessageContent::Text(text)) => {
                    system = Some(text.clone());
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
                        "content": [{
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input,
                        }],
                    }));
                }
                (
                    Role::User,
                    MessageContent::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    },
                ) => {
                    api_messages.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": content,
                            "is_error": is_error,
                        }],
                    }));
                }
                _ => {}
            }
        }

        (system, api_messages)
    }

    fn build_tools(&self, tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.qualified_name(),
                    "description": t.description,
                    "input_schema": t.input_schema,
                })
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl LlmProvider for ClaudeProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> anyhow::Result<LlmResponse> {
        let (system, api_messages) = self.build_messages(messages);
        let api_tools = self.build_tools(tools);

        let mut body = json!({
            "model": self.model,
            "max_tokens": 4096,
            "messages": api_messages,
        });

        if let Some(system_text) = system {
            body["system"] = json!(system_text);
        }

        if !api_tools.is_empty() {
            body["tools"] = json!(api_tools);
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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
            return Err(anyhow::anyhow!("Claude API error ({status}): {error_msg}"));
        }

        // Parse the response content blocks
        let content = response_body["content"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing content in Claude response"))?;

        let mut tool_calls = Vec::new();
        let mut text_parts = Vec::new();

        for block in content {
            match block["type"].as_str() {
                Some("text") => {
                    if let Some(text) = block["text"].as_str() {
                        text_parts.push(text.to_string());
                    }
                }
                Some("tool_use") => {
                    tool_calls.push(ToolCall {
                        id: block["id"].as_str().unwrap_or_default().to_string(),
                        name: block["name"].as_str().unwrap_or_default().to_string(),
                        input: block["input"].clone(),
                    });
                }
                _ => {}
            }
        }

        if !tool_calls.is_empty() {
            Ok(LlmResponse::ToolUse(tool_calls))
        } else {
            Ok(LlmResponse::Text(text_parts.join("\n")))
        }
    }
}
