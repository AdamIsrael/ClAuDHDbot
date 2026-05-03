pub mod commands;

use std::sync::Arc;

use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::llm::LlmProvider;
use crate::mcp::McpManager;
use crate::scheduler::{DmSender, Scheduler};

pub struct Data {
    pub db: SqlitePool,
    pub llm: Box<dyn LlmProvider>,
    pub mcp: Arc<McpManager>,
    pub scheduler: Arc<Mutex<Scheduler>>,
    pub config: Config,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Only allow the configured owner, in DMs or the dev guild.
async fn command_check(ctx: Context<'_>) -> Result<bool, Error> {
    let is_owner = ctx.author().id.get() == ctx.data().config.discord.owner_id;
    let is_dm = ctx.guild_id().is_none();
    let is_dev_guild = match (ctx.guild_id(), ctx.data().config.discord.dev_guild_id) {
        (Some(guild), Some(dev)) => guild.get() == dev,
        _ => false,
    };
    Ok(is_owner && (is_dm || is_dev_guild))
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
            on_error: |error| {
                Box::pin(async move {
                    let ctx_msg = match error.ctx() {
                        Some(ctx) => {
                            let msg = format!("Error: {}", error);
                            let _ = ctx.say(&msg).await;
                            msg
                        }
                        None => format!("{}", error),
                    };
                    tracing::error!("{ctx_msg}");
                })
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                let commands = &framework.options().commands;
                // Always register globally (covers DMs)
                poise::builtins::register_globally(ctx, commands).await?;
                tracing::info!("Commands registered globally");
                // Also register to dev guild for instant updates during development
                if let Some(guild_id) = config.discord.dev_guild_id {
                    let guild = serenity::GuildId::new(guild_id);
                    poise::builtins::register_in_guild(ctx, commands, guild).await?;
                    tracing::info!("Commands also registered to dev guild {guild_id} (instant)");
                }

                let dm_sender: Arc<dyn DmSender> = ctx.http.clone();
                let scheduler = Scheduler::start(&db, dm_sender, config.discord.owner_id).await?;

                let mcp = Arc::new(mcp);

                if config.digest.enabled {
                    if let Err(e) = scheduler
                        .lock()
                        .await
                        .register_digest(db.clone(), mcp.clone(), config.digest.clone())
                        .await
                    {
                        tracing::error!("Failed to register daily digest: {e}");
                    }
                } else {
                    tracing::info!("Daily digest disabled in config");
                }

                tracing::info!("Bot is ready!");
                Ok(Data {
                    db,
                    llm,
                    mcp,
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
