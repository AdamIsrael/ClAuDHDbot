use sqlx::SqlitePool;

use crate::models::schedule::Schedule;

pub async fn create(
    pool: &SqlitePool,
    name: &str,
    cron_expr: &str,
    message: &str,
) -> anyhow::Result<Schedule> {
    let row = sqlx::query_as::<_, Schedule>(
        "INSERT INTO schedules (name, cron_expr, message) VALUES (?, ?, ?) RETURNING *",
    )
    .bind(name)
    .bind(cron_expr)
    .bind(message)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn list(pool: &SqlitePool) -> anyhow::Result<Vec<Schedule>> {
    let rows = sqlx::query_as::<_, Schedule>("SELECT * FROM schedules ORDER BY id")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn list_enabled(pool: &SqlitePool) -> anyhow::Result<Vec<Schedule>> {
    let rows =
        sqlx::query_as::<_, Schedule>("SELECT * FROM schedules WHERE enabled = 1 ORDER BY id")
            .fetch_all(pool)
            .await?;
    Ok(rows)
}

pub async fn get_by_name(pool: &SqlitePool, name: &str) -> anyhow::Result<Option<Schedule>> {
    let row = sqlx::query_as::<_, Schedule>("SELECT * FROM schedules WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn delete(pool: &SqlitePool, id: i64) -> anyhow::Result<bool> {
    let result = sqlx::query("DELETE FROM schedules WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_enabled(pool: &SqlitePool, id: i64, enabled: bool) -> anyhow::Result<bool> {
    let result = sqlx::query("UPDATE schedules SET enabled = ? WHERE id = ?")
        .bind(enabled)
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

        let s = create(&pool, "daily", "0 8 * * *", "good morning")
            .await
            .unwrap();
        assert_eq!(s.name, "daily");
        assert_eq!(s.cron_expr, "0 8 * * *");
        assert_eq!(s.message, "good morning");
        assert!(s.enabled);

        let all = list(&pool).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "daily");
    }

    #[tokio::test]
    async fn test_list_enabled_filters_disabled() {
        let pool = setup_db().await;

        let s1 = create(&pool, "job-a", "0 8 * * *", "msg a").await.unwrap();
        let s2 = create(&pool, "job-b", "0 9 * * *", "msg b").await.unwrap();
        set_enabled(&pool, s2.id, false).await.unwrap();

        let enabled = list_enabled(&pool).await.unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, s1.id);
    }

    #[tokio::test]
    async fn test_list_enabled_empty_when_all_disabled() {
        let pool = setup_db().await;

        let s = create(&pool, "job", "0 * * * *", "ping").await.unwrap();
        set_enabled(&pool, s.id, false).await.unwrap();

        let enabled = list_enabled(&pool).await.unwrap();
        assert!(enabled.is_empty());
    }

    #[tokio::test]
    async fn test_get_by_name_found() {
        let pool = setup_db().await;

        create(&pool, "my-job", "0 * * * *", "ping").await.unwrap();
        let found = get_by_name(&pool, "my-job").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "my-job");
    }

    #[tokio::test]
    async fn test_get_by_name_not_found() {
        let pool = setup_db().await;

        let found = get_by_name(&pool, "nope").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_db().await;

        let s = create(&pool, "bye", "0 * * * *", "farewell").await.unwrap();
        let deleted = delete(&pool, s.id).await.unwrap();
        assert!(deleted);

        let found = get_by_name(&pool, "bye").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_returns_false() {
        let pool = setup_db().await;

        let deleted = delete(&pool, 9999).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_set_enabled_toggle() {
        let pool = setup_db().await;

        let s = create(&pool, "toggle", "0 * * * *", "hi").await.unwrap();
        assert!(s.enabled);

        set_enabled(&pool, s.id, false).await.unwrap();
        let disabled = get_by_name(&pool, "toggle").await.unwrap().unwrap();
        assert!(!disabled.enabled);

        set_enabled(&pool, s.id, true).await.unwrap();
        let re_enabled = get_by_name(&pool, "toggle").await.unwrap().unwrap();
        assert!(re_enabled.enabled);
    }

    #[tokio::test]
    async fn test_set_enabled_nonexistent_returns_false() {
        let pool = setup_db().await;

        let updated = set_enabled(&pool, 9999, false).await.unwrap();
        assert!(!updated);
    }

    #[tokio::test]
    async fn test_create_duplicate_name_fails() {
        let pool = setup_db().await;

        create(&pool, "dup", "0 * * * *", "first").await.unwrap();
        let result = create(&pool, "dup", "0 8 * * *", "second").await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string().to_lowercase();
        assert!(msg.contains("unique"));
    }
}
