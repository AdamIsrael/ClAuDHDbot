use crate::bot::{Context, Error};
use crate::db;
use crate::llm::{ChatMessage, LlmResponse, MessageContent, Role};

/// Manage scheduled jobs.
#[poise::command(
    slash_command,
    prefix_command,
    subcommands("add", "list", "remove", "enable", "disable")
)]
pub async fn schedule(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Ensure a cron expression has 6 fields (tokio-cron-scheduler requires seconds).
/// If given a standard 5-field expression, prepend "0" for seconds.
fn normalize_cron(expr: &str) -> String {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() == 5 {
        format!("0 {expr}")
    } else {
        expr.to_string()
    }
}

/// Convert a natural language schedule to a cron expression using the LLM.
async fn resolve_cron_expr(ctx: &Context<'_>, input: &str) -> Result<String, Error> {
    // If it already looks like a cron expression (5 or 6 fields), use it directly
    let parts: Vec<&str> = input.split_whitespace().collect();
    if (parts.len() == 5 || parts.len() == 6)
        && parts
            .iter()
            .all(|p| p.chars().all(|c| c.is_ascii_digit() || "/*,-".contains(c)))
    {
        return Ok(normalize_cron(input));
    }

    let messages = vec![
        ChatMessage {
            role: Role::System,
            content: MessageContent::Text(
                "Convert the user's schedule description to a standard 5-field cron expression \
                 (minute hour day-of-month month day-of-week). Respond with ONLY the cron \
                 expression, nothing else. Examples:\n\
                 - \"every day at 8am\" -> \"0 8 * * *\"\n\
                 - \"every Monday at 9:30am\" -> \"30 9 * * 1\"\n\
                 - \"every hour\" -> \"0 * * * *\"\n\
                 - \"weekdays at 6pm\" -> \"0 18 * * 1-5\"\n\
                 - \"every 15 minutes\" -> \"*/15 * * * *\"\n\
                 If you cannot parse it, respond with \"ERROR: \" followed by a brief explanation."
                    .to_string(),
            ),
        },
        ChatMessage {
            role: Role::User,
            content: MessageContent::Text(input.to_string()),
        },
    ];

    let response = ctx.data().llm.chat(&messages, &[]).await?;

    match response {
        LlmResponse::Text(text) => {
            let text = text.trim().to_string();
            if text.starts_with("ERROR:") {
                Err(text.into())
            } else {
                Ok(normalize_cron(&text))
            }
        }
        _ => Err("Unexpected LLM response".into()),
    }
}

/// Add a new scheduled job.
#[poise::command(slash_command, prefix_command)]
async fn add(
    ctx: Context<'_>,
    #[description = "Unique name for this schedule"] name: String,
    #[description = "When to run (e.g. 'every day at 8am', 'weekdays at 6pm', or cron)"]
    when: String,
    #[description = "Message to send when triggered"]
    message: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let cron_expr = match resolve_cron_expr(&ctx, &when).await {
        Ok(expr) => expr,
        Err(e) => {
            ctx.say(format!("Couldn't parse schedule: {e}")).await?;
            return Ok(());
        }
    };

    // Validate the cron expression
    if let Err(e) =
        tokio_cron_scheduler::Job::new_async(cron_expr.as_str(), |_uuid, _lock| Box::pin(async {}))
    {
        ctx.say(format!("Invalid cron expression `{cron_expr}`: {e}")).await?;
        return Ok(());
    }

    let data = ctx.data();
    let schedule = db::schedules::create(&data.db, &name, &cron_expr, &message).await;

    match schedule {
        Ok(schedule) => {
            let mut scheduler = data.scheduler.lock().await;
            scheduler.add_job(&schedule).await?;
            ctx.say(format!(
                "Scheduled **{name}** — `{cron_expr}` (from \"{when}\")"
            ))
            .await?;
        }
        Err(e) => {
            let msg = if e.to_string().contains("UNIQUE") {
                format!("A schedule named '{name}' already exists.")
            } else {
                format!("Failed to create schedule: {e}")
            };
            ctx.say(msg).await?;
        }
    }

    Ok(())
}

/// List all scheduled jobs.
#[poise::command(slash_command, prefix_command)]
async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let schedules = db::schedules::list(&ctx.data().db).await?;

    if schedules.is_empty() {
        ctx.say("No scheduled jobs.").await?;
    } else {
        let display: Vec<String> = schedules.iter().map(|s| s.to_string()).collect();
        ctx.say(display.join("\n")).await?;
    }

    Ok(())
}

/// Remove a scheduled job.
#[poise::command(slash_command, prefix_command)]
async fn remove(
    ctx: Context<'_>,
    #[description = "Schedule name"] name: String,
) -> Result<(), Error> {
    let data = ctx.data();

    let schedule = db::schedules::get_by_name(&data.db, &name).await?;
    match schedule {
        Some(s) => {
            let mut scheduler = data.scheduler.lock().await;
            scheduler.remove_job(s.id).await?;
            db::schedules::delete(&data.db, s.id).await?;
            ctx.say(format!("Removed schedule **{name}**.")).await?;
        }
        None => {
            ctx.say(format!("No schedule named '{name}'.")).await?;
        }
    }

    Ok(())
}

/// Enable a scheduled job.
#[poise::command(slash_command, prefix_command)]
async fn enable(
    ctx: Context<'_>,
    #[description = "Schedule name"] name: String,
) -> Result<(), Error> {
    let data = ctx.data();

    let schedule = db::schedules::get_by_name(&data.db, &name).await?;
    match schedule {
        Some(s) if !s.enabled => {
            db::schedules::set_enabled(&data.db, s.id, true).await?;
            let updated = db::schedules::get_by_name(&data.db, &name).await?.unwrap();
            let mut scheduler = data.scheduler.lock().await;
            scheduler.add_job(&updated).await?;
            ctx.say(format!("Enabled schedule **{name}**.")).await?;
        }
        Some(_) => {
            ctx.say(format!("Schedule **{name}** is already enabled."))
                .await?;
        }
        None => {
            ctx.say(format!("No schedule named '{name}'.")).await?;
        }
    }

    Ok(())
}

/// Disable a scheduled job.
#[poise::command(slash_command, prefix_command)]
async fn disable(
    ctx: Context<'_>,
    #[description = "Schedule name"] name: String,
) -> Result<(), Error> {
    let data = ctx.data();

    let schedule = db::schedules::get_by_name(&data.db, &name).await?;
    match schedule {
        Some(s) if s.enabled => {
            let mut scheduler = data.scheduler.lock().await;
            scheduler.remove_job(s.id).await?;
            db::schedules::set_enabled(&data.db, s.id, false).await?;
            ctx.say(format!("Disabled schedule **{name}**.")).await?;
        }
        Some(_) => {
            ctx.say(format!("Schedule **{name}** is already disabled."))
                .await?;
        }
        None => {
            ctx.say(format!("No schedule named '{name}'.")).await?;
        }
    }

    Ok(())
}
