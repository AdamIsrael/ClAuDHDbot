pub mod client;

use std::collections::HashMap;

use async_trait::async_trait;

use crate::config::McpConfig;

pub use client::McpClient;

/// Test seam over a single MCP server connection. Production builds use
/// `McpClient` (rmcp + child-process stdio); tests substitute a mock impl.
#[async_trait]
pub trait McpClientLike: Send + Sync {
    async fn list_tools(&self) -> anyhow::Result<Vec<ToolDescriptor>>;
    async fn call_tool(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value>;
}

/// A single tool's metadata as exposed by an MCP server (no server prefix).
#[derive(Debug, Clone)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Manages connections to multiple MCP servers.
pub struct McpManager {
    clients: HashMap<String, Box<dyn McpClientLike>>,
}

impl McpManager {
    /// Connect to all configured MCP servers. Servers that fail to connect are
    /// logged and skipped — startup proceeds with whatever connected.
    pub async fn connect_all(config: &McpConfig) -> anyhow::Result<Self> {
        let mut clients: HashMap<String, Box<dyn McpClientLike>> = HashMap::new();

        for server in &config.servers {
            match McpClient::connect(server).await {
                Ok(client) => {
                    tracing::info!("Connected to MCP server: {}", server.name);
                    clients.insert(server.name.clone(), Box::new(client));
                }
                Err(e) => {
                    tracing::error!("Failed to connect to MCP server {}: {e}", server.name);
                }
            }
        }

        Ok(Self { clients })
    }

    /// Construct a manager from a preset set of clients. Test seam only.
    #[cfg(test)]
    fn from_clients(clients: HashMap<String, Box<dyn McpClientLike>>) -> Self {
        Self { clients }
    }

    /// List all tools from all connected servers, namespaced as "server.tool".
    /// Servers whose `list_tools` fails are logged and omitted from the result.
    pub async fn list_all_tools(&self) -> Vec<ToolDefinition> {
        let mut all_tools = Vec::new();

        for (server_name, client) in &self.clients {
            match client.list_tools().await {
                Ok(tools) => {
                    for tool in tools {
                        all_tools.push(ToolDefinition {
                            server: server_name.clone(),
                            name: tool.name,
                            description: tool.description,
                            input_schema: tool.input_schema,
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

    /// Call a tool on a specific server. Errors with "Unknown MCP server" if
    /// `server_name` doesn't match a registered client.
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

    /// Parse a namespaced tool name "server.tool" into (server, tool). Splits
    /// on the first `.` so tool names containing dots are preserved.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// In-memory MCP client. Records every `call_tool` invocation so tests
    /// can assert routing.
    struct MockClient {
        tools: Vec<ToolDescriptor>,
        list_should_fail: bool,
        call_response: serde_json::Value,
        call_should_fail: bool,
        calls: Mutex<Vec<(String, serde_json::Value)>>,
    }

    impl MockClient {
        fn with_tools(tools: Vec<ToolDescriptor>) -> Self {
            Self {
                tools,
                list_should_fail: false,
                call_response: serde_json::json!({"content": "", "is_error": false}),
                call_should_fail: false,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn failing_list() -> Self {
            Self {
                tools: Vec::new(),
                list_should_fail: true,
                call_response: serde_json::json!({}),
                call_should_fail: false,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn with_call_response(value: serde_json::Value) -> Self {
            let mut c = Self::with_tools(Vec::new());
            c.call_response = value;
            c
        }

        fn failing_call() -> Self {
            let mut c = Self::with_tools(Vec::new());
            c.call_should_fail = true;
            c
        }

        fn calls(&self) -> Vec<(String, serde_json::Value)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl McpClientLike for MockClient {
        async fn list_tools(&self) -> anyhow::Result<Vec<ToolDescriptor>> {
            if self.list_should_fail {
                anyhow::bail!("mock list_tools failure");
            }
            Ok(self.tools.clone())
        }

        async fn call_tool(
            &self,
            name: &str,
            args: serde_json::Value,
        ) -> anyhow::Result<serde_json::Value> {
            self.calls
                .lock()
                .unwrap()
                .push((name.to_string(), args.clone()));
            if self.call_should_fail {
                anyhow::bail!("mock call_tool failure");
            }
            Ok(self.call_response.clone())
        }
    }

    fn descriptor(name: &str, desc: &str) -> ToolDescriptor {
        ToolDescriptor {
            name: name.to_string(),
            description: desc.to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        }
    }

    fn manager_with(entries: Vec<(&str, Box<dyn McpClientLike>)>) -> McpManager {
        let mut map: HashMap<String, Box<dyn McpClientLike>> = HashMap::new();
        for (name, client) in entries {
            map.insert(name.to_string(), client);
        }
        McpManager::from_clients(map)
    }

    #[tokio::test]
    async fn list_all_tools_namespaces_across_servers() {
        let alpha = MockClient::with_tools(vec![
            descriptor("foo", "alpha-foo"),
            descriptor("bar", "alpha-bar"),
        ]);
        let beta = MockClient::with_tools(vec![descriptor("baz", "beta-baz")]);

        let mgr = manager_with(vec![("alpha", Box::new(alpha)), ("beta", Box::new(beta))]);

        let mut tools = mgr.list_all_tools().await;
        tools.sort_by_key(|t| t.qualified_name());

        let qualified: Vec<String> = tools.iter().map(|t| t.qualified_name()).collect();
        assert_eq!(qualified, vec!["alpha.bar", "alpha.foo", "beta.baz"]);

        let foo = tools.iter().find(|t| t.name == "foo").unwrap();
        assert_eq!(foo.server, "alpha");
        assert_eq!(foo.description, "alpha-foo");
    }

    #[tokio::test]
    async fn list_all_tools_skips_failing_server() {
        let good = MockClient::with_tools(vec![descriptor("ok", "fine")]);
        let bad = MockClient::failing_list();

        let mgr = manager_with(vec![("good", Box::new(good)), ("bad", Box::new(bad))]);

        let tools = mgr.list_all_tools().await;
        assert_eq!(tools.len(), 1, "expected only the working server's tool");
        assert_eq!(tools[0].qualified_name(), "good.ok");
    }

    #[tokio::test]
    async fn list_all_tools_empty_when_no_servers() {
        let mgr = manager_with(vec![]);
        assert!(mgr.list_all_tools().await.is_empty());
    }

    #[tokio::test]
    async fn call_tool_routes_to_correct_server() {
        // We need to look at calls() after the call, but Box<dyn> erases the
        // concrete type. Keep an Arc handle to the mock alongside the boxed
        // trait object for assertions.
        use std::sync::Arc;

        let alpha = Arc::new(MockClient::with_call_response(
            serde_json::json!({"content": "from-alpha", "is_error": false}),
        ));
        let beta = Arc::new(MockClient::with_call_response(
            serde_json::json!({"content": "from-beta", "is_error": false}),
        ));

        // Wrap the Arc in a tiny adapter so it fits Box<dyn McpClientLike>.
        struct ArcAdapter(Arc<MockClient>);
        #[async_trait]
        impl McpClientLike for ArcAdapter {
            async fn list_tools(&self) -> anyhow::Result<Vec<ToolDescriptor>> {
                self.0.list_tools().await
            }
            async fn call_tool(
                &self,
                name: &str,
                args: serde_json::Value,
            ) -> anyhow::Result<serde_json::Value> {
                self.0.call_tool(name, args).await
            }
        }

        let mgr = manager_with(vec![
            ("alpha", Box::new(ArcAdapter(alpha.clone()))),
            ("beta", Box::new(ArcAdapter(beta.clone()))),
        ]);

        let result = mgr
            .call_tool("alpha", "do_thing", serde_json::json!({"x": 1}))
            .await
            .unwrap();
        assert_eq!(result["content"].as_str().unwrap(), "from-alpha");

        // Only alpha received the call.
        assert_eq!(alpha.calls().len(), 1);
        let (name, args) = &alpha.calls()[0];
        assert_eq!(name, "do_thing");
        assert_eq!(args, &serde_json::json!({"x": 1}));
        assert!(beta.calls().is_empty(), "beta should not have been called");
    }

    #[tokio::test]
    async fn call_tool_unknown_server_errors() {
        let mgr = manager_with(vec![(
            "known",
            Box::new(MockClient::with_tools(Vec::new())),
        )]);

        let err = mgr
            .call_tool("missing", "anything", serde_json::json!({}))
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Unknown MCP server"), "msg was: {msg}");
        assert!(msg.contains("missing"), "msg was: {msg}");
    }

    #[tokio::test]
    async fn call_tool_propagates_server_error() {
        let mgr = manager_with(vec![("flaky", Box::new(MockClient::failing_call()))]);

        let err = mgr
            .call_tool("flaky", "anything", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("mock call_tool failure"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_tool_name_splits_at_first_dot() {
        assert_eq!(
            McpManager::parse_tool_name("server.tool"),
            Some(("server", "tool"))
        );
    }

    #[test]
    fn parse_tool_name_keeps_extra_dots_in_tool_part() {
        // First dot wins; everything after stays in the tool name.
        assert_eq!(
            McpManager::parse_tool_name("server.namespace.tool"),
            Some(("server", "namespace.tool"))
        );
    }

    #[test]
    fn parse_tool_name_no_dot_returns_none() {
        assert_eq!(McpManager::parse_tool_name("notnamespaced"), None);
    }

    #[test]
    fn parse_tool_name_empty_string_returns_none() {
        assert_eq!(McpManager::parse_tool_name(""), None);
    }

    #[test]
    fn qualified_name_joins_server_and_tool() {
        let td = ToolDefinition {
            server: "github".to_string(),
            name: "list_repos".to_string(),
            description: "".to_string(),
            input_schema: serde_json::json!({}),
        };
        assert_eq!(td.qualified_name(), "github.list_repos");
    }

    #[tokio::test]
    async fn server_count_reflects_registered_clients() {
        let mgr = manager_with(vec![
            ("a", Box::new(MockClient::with_tools(Vec::new()))),
            ("b", Box::new(MockClient::with_tools(Vec::new()))),
            ("c", Box::new(MockClient::with_tools(Vec::new()))),
        ]);
        assert_eq!(mgr.server_count(), 3);
    }
}
