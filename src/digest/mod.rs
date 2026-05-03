use std::str::FromStr;
use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::{DigestConfig, DigestSectionConfig};
use crate::mcp::McpManager;
use crate::models::task::{Priority, Task};

/// Build the daily digest message.
///
/// The result is always Ok with a formatted string — section failures are
/// logged and rendered inline so the digest still arrives if (e.g.) the
/// calendar MCP is down.
pub async fn build(
    pool: &SqlitePool,
    mcp: Arc<McpManager>,
    config: &DigestConfig,
) -> anyhow::Result<String> {
    let mut sections: Vec<String> = Vec::new();

    sections.push(format!("**Daily digest — {}**", today_str()));

    let min_priority = Priority::from_str(&config.min_priority)?;
    sections.push(tasks_section(pool, min_priority).await?);

    if let Some(cal) = &config.calendar {
        sections.push(mcp_section(&mcp, "Calendar", cal).await);
    }

    Ok(sections.join("\n\n"))
}

async fn tasks_section(pool: &SqlitePool, min_priority: Priority) -> anyhow::Result<String> {
    let tasks = crate::db::tasks::list(pool, None).await?;
    let filtered: Vec<&Task> = tasks
        .iter()
        .filter(|t| {
            Priority::from_str(&t.priority)
                .map(|p| priority_rank(p) >= priority_rank(min_priority))
                .unwrap_or(false)
        })
        .collect();

    if filtered.is_empty() {
        return Ok(format!(
            "**Tasks** ({min_priority}+)\n_No tasks at this priority._"
        ));
    }

    let mut lines = vec![format!("**Tasks** ({min_priority}+)")];
    for task in filtered {
        lines.push(format!("• {task}"));
    }
    Ok(lines.join("\n"))
}

async fn mcp_section(mcp: &McpManager, label: &str, section: &DigestSectionConfig) -> String {
    let header = format!("**{label}**");
    match mcp
        .call_tool(&section.mcp_server, &section.tool, section.args.clone())
        .await
    {
        Ok(value) => {
            let body = value
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if body.is_empty() {
                format!("{header}\n_(empty)_")
            } else {
                format!("{header}\n{body}")
            }
        }
        Err(e) => {
            tracing::warn!("Digest section '{label}' failed: {e}");
            format!("{header}\n_(unavailable: {e})_")
        }
    }
}

fn priority_rank(p: Priority) -> u8 {
    match p {
        Priority::Low => 0,
        Priority::Medium => 1,
        Priority::High => 2,
        Priority::Urgent => 3,
    }
}

fn today_str() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tasks;
    use crate::models::task::Priority;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn tasks_section_includes_high_and_urgent_when_min_high() {
        let pool = setup_db().await;
        tasks::create(&pool, "low task", None, Priority::Low)
            .await
            .unwrap();
        tasks::create(&pool, "med task", None, Priority::Medium)
            .await
            .unwrap();
        tasks::create(&pool, "high task", None, Priority::High)
            .await
            .unwrap();
        tasks::create(&pool, "urgent task", None, Priority::Urgent)
            .await
            .unwrap();

        let out = tasks_section(&pool, Priority::High).await.unwrap();
        assert!(out.contains("high task"), "missing high: {out}");
        assert!(out.contains("urgent task"), "missing urgent: {out}");
        assert!(!out.contains("low task"), "low leaked: {out}");
        assert!(!out.contains("med task"), "med leaked: {out}");
    }

    #[tokio::test]
    async fn tasks_section_excludes_done_tasks() {
        let pool = setup_db().await;
        let t = tasks::create(&pool, "finish me", None, Priority::Urgent)
            .await
            .unwrap();
        tasks::update_status(&pool, t.id, crate::models::task::Status::Done)
            .await
            .unwrap();

        let out = tasks_section(&pool, Priority::High).await.unwrap();
        assert!(!out.contains("finish me"), "done task leaked: {out}");
        assert!(
            out.contains("No tasks"),
            "expected empty placeholder: {out}"
        );
    }

    #[tokio::test]
    async fn tasks_section_empty_renders_placeholder() {
        let pool = setup_db().await;
        let out = tasks_section(&pool, Priority::High).await.unwrap();
        assert!(out.contains("**Tasks**"), "missing header: {out}");
        assert!(out.contains("No tasks"), "missing placeholder: {out}");
    }

    #[tokio::test]
    async fn tasks_section_min_low_includes_everything() {
        let pool = setup_db().await;
        tasks::create(&pool, "low task", None, Priority::Low)
            .await
            .unwrap();
        tasks::create(&pool, "urgent task", None, Priority::Urgent)
            .await
            .unwrap();

        let out = tasks_section(&pool, Priority::Low).await.unwrap();
        assert!(out.contains("low task"));
        assert!(out.contains("urgent task"));
    }

    #[test]
    fn priority_rank_orders_correctly() {
        assert!(priority_rank(Priority::Urgent) > priority_rank(Priority::High));
        assert!(priority_rank(Priority::High) > priority_rank(Priority::Medium));
        assert!(priority_rank(Priority::Medium) > priority_rank(Priority::Low));
    }
}
