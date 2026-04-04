pub mod claude;
pub mod ollama;
pub mod openai;

use crate::config::LlmConfig;
use crate::mcp::ToolDefinition;

/// A message in a conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: MessageContent,
}

#[derive(Debug, Clone)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    /// Tool use requested by the assistant
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Result of a tool call
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

/// Response from an LLM that may request tool calls.
#[derive(Debug)]
pub enum LlmResponse {
    /// Final text response
    Text(String),
    /// LLM wants to call one or more tools before producing a final response
    ToolUse(Vec<ToolCall>),
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a conversation to the LLM, optionally with available tools.
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> anyhow::Result<LlmResponse>;
}

pub fn create_provider(config: &LlmConfig) -> anyhow::Result<Box<dyn LlmProvider>> {
    match config.provider.as_str() {
        "claude" => Ok(Box::new(claude::ClaudeProvider::new(config)?)),
        "ollama" => Ok(Box::new(ollama::OllamaProvider::new(config)?)),
        "openai" => Ok(Box::new(openai::OpenAiProvider::new(config)?)),
        _ => Err(anyhow::anyhow!(
            "Unknown LLM provider: {}. Use: claude, ollama, openai",
            config.provider
        )),
    }
}
