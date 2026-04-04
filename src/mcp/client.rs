use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, Tool},
    service::RunningService,
    transport::TokioChildProcess,
};
use tokio::process::Command;

use crate::config::McpServerConfig;

/// A wrapper around a single MCP server connection.
pub struct McpClient {
    client: RunningService<rmcp::RoleClient, ()>,
}

impl McpClient {
    /// Spawn an MCP server subprocess and connect to it via stdio.
    pub async fn connect(config: &McpServerConfig) -> anyhow::Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);
        for (key, val) in &config.env {
            cmd.env(key, val);
        }

        let transport = TokioChildProcess::new(cmd)?;
        let client = ().serve(transport).await?;

        Ok(Self { client })
    }

    /// List all tools available on this server.
    pub async fn list_tools(&self) -> anyhow::Result<Vec<Tool>> {
        let result = self.client.list_tools(Default::default()).await?;
        Ok(result.tools)
    }

    /// Call a tool by name with the given JSON arguments.
    pub async fn call_tool(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let arguments = args.as_object().cloned();

        let mut params = CallToolRequestParams::new(name.to_string());
        params.arguments = arguments;

        let result = self.client.call_tool(params).await?;

        // Collect all text content from the tool result
        let mut output_parts: Vec<String> = Vec::new();
        for content in &result.content {
            match content.raw {
                rmcp::model::RawContent::Text(ref text) => {
                    output_parts.push(text.text.clone());
                }
                _ => {
                    // For non-text content, serialize it
                    if let Ok(json) = serde_json::to_value(content) {
                        output_parts.push(json.to_string());
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "content": output_parts.join("\n"),
            "is_error": result.is_error.unwrap_or(false),
        }))
    }
}
