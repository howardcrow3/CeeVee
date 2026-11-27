//! Keep-in-touch reminders: who is overdue for a message, based on how close they are.
//!
//! Close people are due after 30 days without contact, warm after 90, weak after 180. The clock
//! starts from the last real contact (a note does not count), or from when the person was added.
//! A person can be put off with `snooze`.

use crate::people::Tier;
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use sqlx::Row;

fn db_err(e: sqlx::Error) -> Error {
    Error::Other(e.to_string())
}

/// Days of silence after which a person of this closeness is due.
pub fn cadence_days(tier: Tier) -> i64 {
    match tier {
        Tier::Close => 30,
        Tier::Warm => 90,
        Tier::Weak => 180,
    }
}

/// Someone who is due for a message.
#[derive(Debug, Clone, PartialEq)]
pub struct Due {
    pub id: i64,
    pub name: String,
    pub tier: Tier,
    pub company: Option<String>,
    /// Whole days since the last contact (or since they were added).
    pub days_silent: i64,
    /// True when you have never been in touch since adding them.
    pub never_contacted: bool,
}

/// People who are overdue at `now` (ISO-8601 UTC), most overdue first. Snoozed people are left
/// out until their date.
pub async fn due(db: &Db, now: &str, limit: i64) -> Result<Vec<Due>> {
    let rows = sqlx::query(
        "SELECT c.id, c.full_name, c.tier, c.last_contacted, \
           CAST(julianday(?1) - julianday(COALESCE(c.last_contacted, c.created_at)) AS INTEGER) AS silent, \
           (SELECT co.name FROM contact_role r JOIN company co ON co.id = r.company_id \
              WHERE r.contact_id = c.id AND r.is_current = 1 ORDER BY r.id DESC LIMIT 1) AS company \
         FROM contact c \
         WHERE (c.snoozed_until IS NULL OR julianday(c.snoozed_until) <= julianday(?1))",
    )
    .bind(now)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    let mut out: Vec<(i64, Due)> = Vec::new();
    for r in rows {
        let tier = match r.get::<String, _>("tier").as_str() {
            "close" => Tier::Close,
            "weak" => Tier::Weak,
            _ => Tier::Warm,
        };
        let silent: i64 = r.get("silent");
        let over = silent - cadence_days(tier);
        if over < 0 {
            continue;
        }
        out.push((
            over,
            Due {
                id: r.get("id"),
                name: r.get("full_name"),
                tier,
                company: r.get("company"),
                days_silent: silent,
                never_contacted: r.get::<Option<String>, _>("last_contacted").is_none(),
            },
        ));
    }
    out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.name.cmp(&b.1.name)));
    Ok(out
        .into_iter()
        .take(limit.max(0) as usize)
        .map(|(_, d)| d)
        .collect())
}

/// Put a person off until `until` (ISO-8601 UTC). Pass `None` to clear it.
pub async fn snooze(db: &Db, id: i64, until: Option<&str>) -> Result<()> {
    let n = sqlx::query("UPDATE contact SET snoozed_until = ? WHERE id = ?")
        .bind(until)
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?
        .rows_affected();
    if n == 0 {
        return Err(Error::NotFound(format!("person {id}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
