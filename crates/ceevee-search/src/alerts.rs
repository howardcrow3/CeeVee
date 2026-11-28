//! New-match alerts: which postings from a search deserve a desktop notification, and what the
//! notification says. Scores run 0 to 100; the cutoff is the `notify.min_score` setting.

use crate::db_err;
use ceevee_core::Result;
use ceevee_db::Db;
use sqlx::Row;

/// Postings scoring below this are stored but never raise a notification.
pub const DEFAULT_MIN_SCORE: f64 = 60.0;
const MIN_SCORE_KEY: &str = "notify.min_score";
const SHOWN: usize = 3;

/// A posting worth telling the user about.
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub title: String,
    pub company: Option<String>,
    pub score: f64,
}

/// The cutoff for alerts: the saved setting, else [`DEFAULT_MIN_SCORE`].
pub async fn min_score(db: &Db) -> f64 {
    match db.get_setting(MIN_SCORE_KEY).await {
        Ok(Some(v)) => v.trim().parse().unwrap_or(DEFAULT_MIN_SCORE),
        _ => DEFAULT_MIN_SCORE,
    }
}

/// Postings first seen at or after `since` (ISO-8601 UTC) that you have not saved or dismissed
/// and that score at least `min_score`, best first.
pub async fn new_matches(db: &Db, since: &str, min_score: f64) -> Result<Vec<Match>> {
    let rows = sqlx::query(
        "SELECT p.title, c.name AS company, p.score FROM posting p \
         LEFT JOIN company c ON c.id = p.company_id \
         WHERE p.first_seen >= ? AND p.state = 'new' AND p.score >= ? \
         ORDER BY p.score DESC, p.id",
    )
    .bind(since)
    .bind(min_score)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    Ok(rows
        .iter()
        .map(|r| Match {
            title: r.get("title"),
            company: r.get("company"),
            score: r.get("score"),
        })
        .collect())
}

/// How many postings arrived at or after `since`, for the tray menu's "New jobs today".
pub async fn count_since(db: &Db, since: &str) -> Result<i64> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM posting WHERE first_seen >= ? AND state != 'dismissed'",
    )
    .bind(since)
    .fetch_one(db.pool())
    .await
    .map_err(db_err)
}

fn describe(m: &Match) -> String {
    match &m.company {
        Some(c) => format!("{} at {c}", m.title),
        None => m.title.clone(),
    }
}

/// The notification title and body for these matches, or `None` when there is nothing to say.
pub fn message(matches: &[Match]) -> Option<(String, String)> {
    let n = matches.len();
    if n == 0 {
        return None;
    }
    let title = if n == 1 {
        "1 new job matches your search".to_string()
    } else {
        format!("{n} new jobs match your search")
    };
    let mut body = matches
        .iter()
        .take(SHOWN)
        .map(describe)
        .collect::<Vec<_>>()
        .join("; ");
    if n > SHOWN {
        body.push_str(&format!("; and {} more", n - SHOWN));
    }
    Some((title, body))
}

#[cfg(test)]
mod tests;
