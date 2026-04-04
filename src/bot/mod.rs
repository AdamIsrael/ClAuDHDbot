pub mod commands;

use std::sync::Arc;

use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::llm::LlmProvider;
use crate::mcp::McpManager;
use crate::scheduler::Scheduler;

pub struct Data {
    pub db: SqlitePool,
    pub llm: Box<dyn LlmProvider>,
    pub mcp: Arc<McpManager>,
    pub scheduler: Arc<Mutex<Scheduler>>,
    pub config: Config,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Only allow the configured owner, and only in DMs.
async fn command_check(ctx: Context<'_>) -> Result<bool, Error> {
    let is_owner = ctx.author().id.get() == ctx.data().config.discord.owner_id;
    let is_dm = ctx.guild_id().is_none();
    Ok(is_owner && is_dm)
}

pub async fn run(
    config: Config,
    db: SqlitePool,
    llm: Box<dyn LlmProvider>,
    mcp: McpManager,
) -> anyhow::Result<()> {
    let token = config.discord.token.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::ping(),
                commands::task(),
                commands::ask(),
                commands::tools(),
                commands::schedule(),
            ],
            command_check: Some(|ctx| Box::pin(command_check(ctx))),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let http = ctx.http.clone();
                let scheduler =
                    Scheduler::start(&db, http, config.discord.owner_id).await?;

                tracing::info!("Bot is ready!");
                Ok(Data {
                    db,
                    llm,
                    mcp: Arc::new(mcp),
                    scheduler,
                    config,
                })
            })
        })
        .build();

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::ClientBuilder::new(&token, intents)
        .framework(framework)
        .await?;

    client.start().await?;

    Ok(())
}
