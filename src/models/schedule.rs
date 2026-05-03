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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_schedule(enabled: bool) -> Schedule {
        Schedule {
            id: 1,
            name: "daily-digest".to_string(),
            cron_expr: "0 8 * * *".to_string(),
            message: "Good morning!".to_string(),
            enabled,
            created_at: "2026-01-01T00:00:00".to_string(),
        }
    }

    #[test]
    fn test_display_enabled() {
        let s = make_schedule(true);
        let out = s.to_string();
        assert!(out.contains("**daily-digest**"), "name missing: {out}");
        assert!(out.contains("`0 8 * * *`"), "cron missing: {out}");
        assert!(out.contains("(on)"), "status missing: {out}");
        assert!(out.contains("Good morning!"), "message missing: {out}");
    }

    #[test]
    fn test_display_disabled() {
        let s = make_schedule(false);
        let out = s.to_string();
        assert!(out.contains("(off)"), "expected (off): {out}");
        assert!(!out.contains("(on)"), "should not contain (on): {out}");
    }

    #[test]
    fn test_display_format_structure() {
        let s = make_schedule(true);
        let out = s.to_string();
        // Verify the em-dash separator is present
        assert!(out.contains("—"), "em-dash separator missing: {out}");
    }
}
