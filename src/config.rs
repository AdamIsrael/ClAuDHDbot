use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub discord: DiscordConfig,
    pub llm: LlmConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub mcp: McpConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiscordConfig {
    pub token: String,
    pub owner_id: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

pub fn load() -> anyhow::Result<Config> {
    // Load .env file if present (vars won't override existing env)
    let _ = dotenvy::dotenv();

    let config = Figment::new()
        .merge(Toml::file("config/default.toml"))
        .merge(Toml::file("config/mcp_servers.toml"))
        .merge(Env::prefixed("CLAUDHD_").split("__"))
        .extract()?;
    Ok(config)
}
