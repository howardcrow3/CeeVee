//! How networking is going: a weekly goal, how many asks turned into replies and referrals, and
//! people who just changed jobs (a good reason to say congratulations, with nothing to ask for).

use crate::people::{self, Change};
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use sqlx::Row;

const GOAL_KEY: &str = "network.weekly_goal";
const DEFAULT_GOAL: i64 = 3;

fn db_err(e: sqlx::Error) -> Error {
    Error::Other(e.to_string())
}

/// People to be in touch with each week. Three unless the person changed it.
pub async fn weekly_goal(db: &Db) -> Result<i64> {
    let v: Option<String> = sqlx::query_scalar("SELECT value FROM setting WHERE key = ?")
        .bind(GOAL_KEY)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?;
    Ok(v.and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_GOAL))
}

pub async fn set_weekly_goal(db: &Db, n: i64) -> Result<()> {
    if !(1..=50).contains(&n) {
        return Err(Error::Invalid("pick a goal between 1 and 50 a week".into()));
    }
    sqlx::query(
        "INSERT INTO setting (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(GOAL_KEY)
    .bind(n.to_string())
    .execute(db.pool())
    .await
    .map_err(db_err)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub goal: i64,
    /// Different people you were in touch with in the last 7 days. Notes do not count.
    pub this_week: i64,
    /// Asks for introductions in the last 30 days, and how they went.
    pub asked: i64,
    pub replied: i64,
    pub referred: i64,
    /// People who changed jobs in the last 30 days.
    pub moves: Vec<Change>,
}

pub async fn summary(db: &Db, now: &str) -> Result<Progress> {
    let this_week: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT contact_id) FROM interaction \
          WHERE kind <> 'note' AND julianday(?1) - julianday(at) BETWEEN 0 AND 7",
    )
    .bind(now)
    .fetch_one(db.pool())
    .await
    .map_err(db_err)?;
    let rows = sqlx::query(
        "SELECT status, COUNT(*) AS n FROM outreach \
          WHERE julianday(?1) - julianday(asked_at) BETWEEN 0 AND 30 GROUP BY status",
    )
    .bind(now)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    let count = |s: &str| {
        rows.iter()
            .find(|r| r.get::<String, _>("status") == s)
            .map_or(0, |r| r.get::<i64, _>("n"))
    };
    let (asked_only, replied, referred) = (count("asked"), count("replied"), count("referred"));
    let mut moves = Vec::new();
    for c in people::recent_changes(db, 50).await? {
        let recent: Option<i64> =
            sqlx::query_scalar("SELECT julianday(?1) - julianday(?2) BETWEEN 0 AND 30")
                .bind(now)
                .bind(&c.seen_at)
                .fetch_one(db.pool())
                .await
                .map_err(db_err)?;
        if recent == Some(1) {
            moves.push(c);
        }
    }
    Ok(Progress {
        goal: weekly_goal(db).await?,
        this_week,
        asked: asked_only + replied + referred,
        replied: replied + referred,
        referred,
        moves,
    })
}

#[cfg(test)]
mod tests;
