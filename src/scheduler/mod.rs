use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::config::DigestConfig;
use crate::mcp::McpManager;
use crate::models::schedule::Schedule;

/// Test seam over the side-effect of sending a DM. Real builds use
/// `serenity::Http`; tests substitute a capturing mock.
#[async_trait]
pub trait DmSender: Send + Sync {
    async fn send_dm(&self, user_id: u64, message: &str) -> anyhow::Result<()>;
}

#[async_trait]
impl DmSender for serenity::Http {
    async fn send_dm(&self, user_id: u64, message: &str) -> anyhow::Result<()> {
        let channel = serenity::UserId::new(user_id)
            .create_dm_channel(self)
            .await?;
        channel.say(self, message).await?;
        Ok(())
    }
}

pub struct Scheduler {
    job_scheduler: JobScheduler,
    /// Maps schedule DB id → cron job UUID (for removal)
    job_ids: HashMap<i64, uuid::Uuid>,
    dm_sender: Arc<dyn DmSender>,
    owner_id: u64,
}

impl Scheduler {
    /// Create and start the scheduler, loading all enabled schedules from the DB.
    pub async fn start(
        pool: &SqlitePool,
        dm_sender: Arc<dyn DmSender>,
        owner_id: u64,
    ) -> anyhow::Result<Arc<Mutex<Self>>> {
        let job_scheduler = JobScheduler::new().await?;

        let mut scheduler = Self {
            job_scheduler,
            job_ids: HashMap::new(),
            dm_sender,
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
        let dm_sender = self.dm_sender.clone();
        let owner_id = self.owner_id;
        let message = schedule.message.clone();
        let name = schedule.name.clone();

        let job = Job::new_async(schedule.cron_expr.as_str(), move |_uuid, _lock| {
            let dm_sender = dm_sender.clone();
            let message = message.clone();
            let name = name.clone();
            Box::pin(async move {
                if let Err(e) = dm_sender.send_dm(owner_id, &message).await {
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

    /// Register the recurring Daily Digest job. The digest is built fresh on
    /// each fire from current DB state and the configured MCP sections.
    pub async fn register_digest(
        &mut self,
        pool: SqlitePool,
        mcp: Arc<McpManager>,
        digest_config: DigestConfig,
    ) -> anyhow::Result<()> {
        let dm_sender = self.dm_sender.clone();
        let owner_id = self.owner_id;
        let cron = digest_config.cron.clone();

        let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
            let dm_sender = dm_sender.clone();
            let pool = pool.clone();
            let mcp = mcp.clone();
            let cfg = digest_config.clone();
            Box::pin(async move {
                match crate::digest::build(&pool, mcp, &cfg).await {
                    Ok(message) => {
                        if let Err(e) = dm_sender.send_dm(owner_id, &message).await {
                            tracing::error!("Daily digest failed to send DM: {e}");
                        }
                    }
                    Err(e) => tracing::error!("Daily digest build failed: {e}"),
                }
            })
        })?;

        self.job_scheduler.add(job).await?;
        tracing::info!("Daily digest registered (cron: {cron})");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;
    use std::time::Duration;

    /// In-memory DmSender that records every call.
    #[derive(Default)]
    struct MockDmSender {
        sent: StdMutex<Vec<(u64, String)>>,
    }

    impl MockDmSender {
        fn count(&self) -> usize {
            self.sent.lock().unwrap().len()
        }

        fn snapshot(&self) -> Vec<(u64, String)> {
            self.sent.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl DmSender for MockDmSender {
        async fn send_dm(&self, user_id: u64, message: &str) -> anyhow::Result<()> {
            self.sent
                .lock()
                .unwrap()
                .push((user_id, message.to_string()));
            Ok(())
        }
    }

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    /// Cron expression that fires every second.
    const EVERY_SECOND: &str = "* * * * * *";

    /// Window long enough for tokio-cron-scheduler to fire at least once.
    const FIRE_WINDOW: Duration = Duration::from_millis(2200);

    const OWNER_ID: u64 = 12345;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn enabled_schedules_load_on_start_and_fire() {
        let pool = setup_db().await;
        crate::db::schedules::create(&pool, "preloaded", EVERY_SECOND, "tick")
            .await
            .unwrap();

        let mock = Arc::new(MockDmSender::default());
        let _scheduler = Scheduler::start(&pool, mock.clone(), OWNER_ID)
            .await
            .unwrap();

        tokio::time::sleep(FIRE_WINDOW).await;

        let snap = mock.snapshot();
        assert!(!snap.is_empty(), "expected at least one DM, got 0");
        let (uid, msg) = &snap[0];
        assert_eq!(*uid, OWNER_ID);
        assert_eq!(msg, "tick");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn disabled_schedules_do_not_load() {
        let pool = setup_db().await;
        let s = crate::db::schedules::create(&pool, "off", EVERY_SECOND, "should not fire")
            .await
            .unwrap();
        crate::db::schedules::set_enabled(&pool, s.id, false)
            .await
            .unwrap();

        let mock = Arc::new(MockDmSender::default());
        let _scheduler = Scheduler::start(&pool, mock.clone(), OWNER_ID)
            .await
            .unwrap();

        tokio::time::sleep(FIRE_WINDOW).await;

        assert_eq!(mock.count(), 0, "disabled schedule fired");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_job_at_runtime_fires_then_remove_stops_it() {
        let pool = setup_db().await;
        let mock = Arc::new(MockDmSender::default());
        let scheduler = Scheduler::start(&pool, mock.clone(), OWNER_ID)
            .await
            .unwrap();

        // Insert + register at runtime (mirrors what /schedule add does).
        let schedule = crate::db::schedules::create(&pool, "runtime", EVERY_SECOND, "ping")
            .await
            .unwrap();
        scheduler.lock().await.add_job(&schedule).await.unwrap();

        tokio::time::sleep(FIRE_WINDOW).await;
        let count_before_remove = mock.count();
        assert!(
            count_before_remove >= 1,
            "expected ≥1 DM after add_job, got {count_before_remove}"
        );

        scheduler
            .lock()
            .await
            .remove_job(schedule.id)
            .await
            .unwrap();

        // After remove, allow any in-flight fire to settle then check the
        // count stops growing.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let count_after_settle = mock.count();
        tokio::time::sleep(FIRE_WINDOW).await;
        let count_final = mock.count();
        assert_eq!(
            count_final, count_after_settle,
            "remove_job did not stop further fires (after settle: {count_after_settle}, final: {count_final})"
        );

        // Sanity: every captured message has the right user + body.
        for (uid, msg) in mock.snapshot() {
            assert_eq!(uid, OWNER_ID);
            assert_eq!(msg, "ping");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remove_job_unknown_id_is_ok() {
        let pool = setup_db().await;
        let mock = Arc::new(MockDmSender::default());
        let scheduler = Scheduler::start(&pool, mock, OWNER_ID).await.unwrap();

        // Removing an id that was never registered is a no-op, not an error.
        scheduler.lock().await.remove_job(9999).await.unwrap();
    }
}
