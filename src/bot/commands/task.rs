use crate::bot::{Context, Error};
use crate::db;
use crate::models::{Priority, Status};

/// Manage your tasks.
#[poise::command(
    slash_command,
    prefix_command,
    subcommands("add", "list", "done", "claim", "release", "delete")
)]
pub async fn task(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add a new task.
#[poise::command(slash_command, prefix_command)]
async fn add(
    ctx: Context<'_>,
    #[description = "Task title"] title: String,
    #[description = "Priority: low, medium, high, urgent"] priority: Option<String>,
    #[description = "Task description"] description: Option<String>,
) -> Result<(), Error> {
    let priority = match &priority {
        Some(p) => p.parse::<Priority>()?,
        None => Priority::Medium,
    };

    let task = db::tasks::create(&ctx.data().db, &title, description.as_deref(), priority).await?;

    ctx.say(format!("Created task #{}: {}", task.id, task.title))
        .await?;
    Ok(())
}

/// List tasks.
#[poise::command(slash_command, prefix_command)]
async fn list(
    ctx: Context<'_>,
    #[description = "Filter by status: pending, in_progress, done"] status: Option<String>,
) -> Result<(), Error> {
    let status = match &status {
        Some(s) => Some(s.parse::<Status>()?),
        None => None,
    };

    let tasks = db::tasks::list(&ctx.data().db, status).await?;

    if tasks.is_empty() {
        ctx.say("No tasks found.").await?;
    } else {
        let display: Vec<String> = tasks.iter().map(|t| t.to_string()).collect();
        ctx.say(display.join("\n")).await?;
    }

    Ok(())
}

/// Mark a task as done.
#[poise::command(slash_command, prefix_command)]
async fn done(ctx: Context<'_>, #[description = "Task ID"] id: i64) -> Result<(), Error> {
    if db::tasks::update_status(&ctx.data().db, id, Status::Done).await? {
        ctx.say(format!("Task #{id} marked as done.")).await?;
    } else {
        ctx.say(format!("Task #{id} not found.")).await?;
    }
    Ok(())
}

/// Claim a task (mark as in progress).
#[poise::command(slash_command, prefix_command)]
async fn claim(ctx: Context<'_>, #[description = "Task ID"] id: i64) -> Result<(), Error> {
    if db::tasks::update_status(&ctx.data().db, id, Status::InProgress).await? {
        ctx.say(format!("Task #{id} claimed.")).await?;
    } else {
        ctx.say(format!("Task #{id} not found.")).await?;
    }
    Ok(())
}

/// Release a task (mark as pending).
#[poise::command(slash_command, prefix_command)]
async fn release(ctx: Context<'_>, #[description = "Task ID"] id: i64) -> Result<(), Error> {
    if db::tasks::update_status(&ctx.data().db, id, Status::Pending).await? {
        ctx.say(format!("Task #{id} released.")).await?;
    } else {
        ctx.say(format!("Task #{id} not found.")).await?;
    }
    Ok(())
}

/// Delete a task.
#[poise::command(slash_command, prefix_command)]
async fn delete(ctx: Context<'_>, #[description = "Task ID"] id: i64) -> Result<(), Error> {
    if db::tasks::delete(&ctx.data().db, id).await? {
        ctx.say(format!("Task #{id} deleted.")).await?;
    } else {
        ctx.say(format!("Task #{id} not found.")).await?;
    }
    Ok(())
}
