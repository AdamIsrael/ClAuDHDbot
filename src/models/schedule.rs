use std::fmt;

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Schedule {
    pub id: i64,
    pub name: String,
    pub cron_expr: String,
    pub message: String,
    pub enabled: bool,
    pub created_at: String,
}

impl fmt::Display for Schedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.enabled { "on" } else { "off" };
        write!(
            f,
            "**{name}** [`{cron}`] ({status}) — {message}",
            name = self.name,
            cron = self.cron_expr,
            message = self.message,
        )
    }
}
