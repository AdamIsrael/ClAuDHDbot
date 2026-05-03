use std::path::PathBuf;

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
    #[serde(default)]
    pub digest: DigestConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiscordConfig {
    pub token: String,
    pub owner_id: u64,
    /// When set, commands register to this guild instantly (for development).
    /// Without this, global commands can take up to an hour to propagate.
    pub dev_guild_id: Option<u64>,
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

/// Daily Digest: a recurring DM summarising tasks and (optionally) calendar.
#[derive(Debug, Deserialize, Clone)]
pub struct DigestConfig {
    /// When false (default), the digest does not run.
    #[serde(default)]
    pub enabled: bool,
    /// Six-field cron expression (with seconds). Default: 8am daily.
    #[serde(default = "default_digest_cron")]
    pub cron: String,
    /// Lowest priority of task to include in the digest. Default: "high".
    /// Valid values: "low", "medium", "high", "urgent".
    #[serde(default = "default_min_priority")]
    pub min_priority: String,
    /// Optional calendar section: invokes an MCP tool and appends its text output.
    pub calendar: Option<DigestSectionConfig>,
}

impl Default for DigestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cron: default_digest_cron(),
            min_priority: default_min_priority(),
            calendar: None,
        }
    }
}

/// One MCP-backed section of the digest (e.g. calendar, weather).
#[derive(Debug, Deserialize, Clone)]
pub struct DigestSectionConfig {
    /// MCP server name as configured under `[[mcp.servers]]`.
    pub mcp_server: String,
    /// Tool name on that server (without the `server.` prefix).
    pub tool: String,
    /// JSON arguments passed to the tool. Defaults to `{}`.
    #[serde(default = "default_section_args")]
    pub args: serde_json::Value,
}

fn default_digest_cron() -> String {
    "0 0 8 * * *".to_string()
}

fn default_min_priority() -> String {
    "high".to_string()
}

fn default_section_args() -> serde_json::Value {
    serde_json::json!({})
}

pub fn load() -> anyhow::Result<Config> {
    // Load .env file if present (vars won't override existing env)
    let _ = dotenvy::dotenv();

    let config_dir = config_dir()?;
    let config = Figment::new()
        .merge(Toml::file(config_dir.join("default.toml")))
        .merge(Toml::file(config_dir.join("mcp_servers.toml")))
        .merge(Env::prefixed("CLAUDHD_").split("__"))
        .extract()?;
    Ok(config)
}

/// Resolve the config directory: `$CLAUDHD_CONFIG_DIR` if set, otherwise
/// `$XDG_CONFIG_HOME/claudhdbot` (falling back to `$HOME/.config/claudhdbot`).
fn config_dir() -> anyhow::Result<PathBuf> {
    if let Ok(dir) = std::env::var("CLAUDHD_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let base = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("could not determine user config directory"))?;
    Ok(base.join("claudhdbot"))
}
