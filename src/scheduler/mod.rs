use std::collections::HashMap;
use std::sync::Arc;

use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::models::schedule::Schedule;

pub struct Scheduler {
    job_scheduler: JobScheduler,
    /// Maps schedule DB id → cron job UUID (for removal)
    job_ids: HashMap<i64, uuid::Uuid>,
    http: Arc<serenity::Http>,
    owner_id: serenity::UserId,
}

impl Scheduler {
    /// Create and start the scheduler, loading all enabled schedules from the DB.
    pub async fn start(
        pool: &SqlitePool,
        http: Arc<serenity::Http>,
        owner_id: u64,
    ) -> anyhow::Result<Arc<Mutex<Self>>> {
        let job_scheduler = JobScheduler::new().await?;
        let owner_id = serenity::UserId::new(owner_id);

        let mut scheduler = Self {
            job_scheduler,
            job_ids: HashMap::new(),
            http,
            owner_id,
        };

        // Load existing enabled schedules
        let schedules = crate::db::schedules::list_enabled(pool).await?;
        for schedule in &schedules {
            if let Err(e) = scheduler.add_job(schedule).await {
                tracing::error!("Failed to load schedule '{}': {e}", schedule.name);
            }
        }

        scheduler.job_scheduler.start().await?;
        tracing::info!("Scheduler started with {} job(s)", scheduler.job_ids.len());

        Ok(Arc::new(Mutex::new(scheduler)))
    }

    /// Add a job for a schedule. The job DMs the owner with the schedule's message.
    pub async fn add_job(&mut self, schedule: &Schedule) -> anyhow::Result<()> {
        let http = self.http.clone();
        let owner_id = self.owner_id;
        let message = schedule.message.clone();
        let name = schedule.name.clone();

        let job = Job::new_async(schedule.cron_expr.as_str(), move |_uuid, _lock| {
            let http = http.clone();
            let message = message.clone();
            let name = name.clone();
            Box::pin(async move {
                if let Err(e) = send_dm(&http, owner_id, &message).await {
                    tracing::error!("Scheduled job '{name}' failed to send DM: {e}");
                }
            })
        })?;

        let uuid = self.job_scheduler.add(job).await?;
        self.job_ids.insert(schedule.id, uuid);

        Ok(())
    }

    /// Remove a running job by schedule DB id.
    pub async fn remove_job(&mut self, schedule_id: i64) -> anyhow::Result<()> {
        if let Some(uuid) = self.job_ids.remove(&schedule_id) {
            self.job_scheduler.remove(&uuid).await?;
        }
        Ok(())
    }
}

async fn send_dm(
    http: &serenity::Http,
    user_id: serenity::UserId,
    message: &str,
) -> anyhow::Result<()> {
    let channel = user_id.create_dm_channel(http).await?;
    channel.say(http, message).await?;
    Ok(())
}
