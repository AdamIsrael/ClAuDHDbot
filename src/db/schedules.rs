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
