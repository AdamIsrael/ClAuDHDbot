mod bot;
mod config;
mod db;
mod llm;
mod mcp;
mod models;
mod scheduler;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("claudhdbot=info")),
        )
        .init();

    let config = config::load()?;
    tracing::info!("Configuration loaded");

    let db = db::init(&config.database).await?;
    tracing::info!("Database initialized");

    let mcp = mcp::McpManager::connect_all(&config.mcp).await?;
    tracing::info!("Connected to {} MCP server(s)", mcp.server_count());

    let llm = llm::create_provider(&config.llm)?;
    tracing::info!("LLM provider: {}", config.llm.provider);

    bot::run(config, db, llm, mcp).await?;

    Ok(())
}
