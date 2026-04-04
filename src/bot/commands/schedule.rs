use crate::bot::{Context, Error};
use crate::db;

/// Manage scheduled jobs.
#[poise::command(
    slash_command,
    prefix_command,
    subcommands("add", "list", "remove", "enable", "disable")
)]
pub async fn schedule(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add a new scheduled job.
#[poise::command(slash_command, prefix_command)]
async fn add(
    ctx: Context<'_>,
    #[description = "Unique name for this schedule"] name: String,
    #[description = "Cron expression (e.g. '0 8 * * *' for daily at 8am UTC)"] cron_expr: String,
    #[description = "Message to send when triggered"]
    #[rest]
    message: String,
) -> Result<(), Error> {
    // Validate cron expression by trying to create a job
    if let Err(e) =
        tokio_cron_scheduler::Job::new_async(cron_expr.as_str(), |_uuid, _lock| Box::pin(async {}))
    {
        ctx.say(format!("Invalid cron expression: {e}")).await?;
        return Ok(());
    }

    let data = ctx.data();
    let schedule = db::schedules::create(&data.db, &name, &cron_expr, &message).await;

    match schedule {
        Ok(schedule) => {
            // Add to the live scheduler
            let mut scheduler = data.scheduler.lock().await;
            scheduler.add_job(&schedule).await?;
            ctx.say(format!("Scheduled **{name}** with cron `{cron_expr}`"))
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
