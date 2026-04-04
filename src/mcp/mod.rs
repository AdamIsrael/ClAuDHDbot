pub mod client;

use std::collections::HashMap;

use crate::config::McpConfig;

pub use client::McpClient;
use serde_json::Value as JsonValue;

/// Manages connections to multiple MCP servers.
pub struct McpManager {
    clients: HashMap<String, McpClient>,
}

impl McpManager {
    /// Connect to all configured MCP servers.
    pub async fn connect_all(config: &McpConfig) -> anyhow::Result<Self> {
        let mut clients = HashMap::new();

        for server in &config.servers {
            match McpClient::connect(server).await {
                Ok(client) => {
                    tracing::info!("Connected to MCP server: {}", server.name);
                    clients.insert(server.name.clone(), client);
                }
                Err(e) => {
                    tracing::error!("Failed to connect to MCP server {}: {e}", server.name);
                }
            }
        }

        Ok(Self { clients })
    }

    /// List all tools from all connected servers, namespaced as "server.tool".
    pub async fn list_all_tools(&self) -> Vec<ToolDefinition> {
        let mut all_tools = Vec::new();

        for (server_name, client) in &self.clients {
            match client.list_tools().await {
                Ok(tools) => {
                    for tool in tools {
                        // Convert Arc<JsonObject> to serde_json::Value
                        let schema = JsonValue::Object((*tool.input_schema).clone());
                        all_tools.push(ToolDefinition {
                            server: server_name.clone(),
                            name: tool.name.to_string(),
                            description: tool.description.unwrap_or_default().to_string(),
                            input_schema: schema,
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to list tools from {server_name}: {e}");
                }
            }
        }

        all_tools
    }

    /// Call a tool on a specific server.
    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let client = self
            .clients
            .get(server_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown MCP server: {server_name}"))?;

        client.call_tool(tool_name, args).await
    }

    /// Parse a namespaced tool name "server.tool" into (server, tool).
    pub fn parse_tool_name(namespaced: &str) -> Option<(&str, &str)> {
        namespaced.split_once('.')
    }

    pub fn server_count(&self) -> usize {
        self.clients.len()
    }
}

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub server: String,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl ToolDefinition {
    /// Fully qualified name: "server.tool"
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.server, self.name)
    }
}
