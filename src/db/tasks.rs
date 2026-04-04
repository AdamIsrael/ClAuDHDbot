use sqlx::SqlitePool;

use crate::models::task::{Priority, Status, Task};

pub async fn create(
    pool: &SqlitePool,
    title: &str,
    description: Option<&str>,
    priority: Priority,
) -> anyhow::Result<Task> {
    let priority_str = priority.to_string();
    let row = sqlx::query_as::<_, Task>(
        "INSERT INTO tasks (title, description, priority) VALUES (?, ?, ?) RETURNING *",
    )
    .bind(title)
    .bind(description)
    .bind(&priority_str)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn list(pool: &SqlitePool, status: Option<Status>) -> anyhow::Result<Vec<Task>> {
    let tasks = match status {
        Some(s) => {
            let status_str = s.to_string();
            sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE status = ? ORDER BY id")
                .bind(&status_str)
                .fetch_all(pool)
                .await?
        }
        None => {
            sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE status != 'done' ORDER BY id")
                .fetch_all(pool)
                .await?
        }
    };

    Ok(tasks)
}

pub async fn update_status(pool: &SqlitePool, id: i64, status: Status) -> anyhow::Result<bool> {
    let status_str = status.to_string();
    let result =
        sqlx::query("UPDATE tasks SET status = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(&status_str)
            .bind(id)
            .execute(pool)
            .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete(pool: &SqlitePool, id: i64) -> anyhow::Result<bool> {
    let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let pool = setup_db().await;

        let task = create(&pool, "Buy groceries", None, Priority::High)
            .await
            .unwrap();
        assert_eq!(task.title, "Buy groceries");
        assert_eq!(task.priority, "high");
        assert_eq!(task.status, "pending");

        let tasks = list(&pool, None).await.unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_update_status() {
        let pool = setup_db().await;

        let task = create(&pool, "Test task", None, Priority::Medium)
            .await
            .unwrap();
        let updated = update_status(&pool, task.id, Status::Done).await.unwrap();
        assert!(updated);

        // Done tasks excluded from default list
        let tasks = list(&pool, None).await.unwrap();
        assert_eq!(tasks.len(), 0);

        // But visible when filtering by done
        let done = list(&pool, Some(Status::Done)).await.unwrap();
        assert_eq!(done.len(), 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_db().await;

        let task = create(&pool, "Delete me", None, Priority::Low)
            .await
            .unwrap();
        let deleted = delete(&pool, task.id).await.unwrap();
        assert!(deleted);

        let tasks = list(&pool, None).await.unwrap();
        assert_eq!(tasks.len(), 0);
    }
}
