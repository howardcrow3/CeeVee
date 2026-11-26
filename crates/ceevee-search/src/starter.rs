//! The starter feed: a built-in set of well-known company job boards, so a first search shows real
//! openings without the person having to know a board token. They are searched along with the
//! boards the person added, and scored against their job types like any other posting.
//!
//! The list is a convenience, not a promise: a company can move boards or close one. A starter
//! board that cannot be read is skipped quietly (and logged). Only the person's own boards report
//! failures. The person can turn the starter feed off with [`set_enabled`].

use crate::sources::{self, Board, BoardKind, Fetcher, SearchSummary};
use ceevee_core::Result;
use ceevee_db::Db;
use sqlx::Row;

const SETTING: &str = "search.starter";

use BoardKind::{Greenhouse as G, Lever as L};

/// (kind, board token, company name)
const STARTER: &[(BoardKind, &str, &str)] = &[
    (G, "stripe", "Stripe"),
    (G, "airbnb", "Airbnb"),
    (G, "figma", "Figma"),
    (G, "discord", "Discord"),
    (G, "databricks", "Databricks"),
    (G, "cloudflare", "Cloudflare"),
    (G, "reddit", "Reddit"),
    (G, "robinhood", "Robinhood"),
    (G, "coinbase", "Coinbase"),
    (G, "instacart", "Instacart"),
    (G, "pinterest", "Pinterest"),
    (G, "dropbox", "Dropbox"),
    (G, "datadog", "Datadog"),
    (G, "asana", "Asana"),
    (G, "gitlab", "GitLab"),
    (G, "mongodb", "MongoDB"),
    (G, "twilio", "Twilio"),
    (G, "lyft", "Lyft"),
    (G, "anthropic", "Anthropic"),
    (G, "duolingo", "Duolingo"),
    (G, "brex", "Brex"),
    (G, "gusto", "Gusto"),
    (G, "okta", "Okta"),
    (G, "vercel", "Vercel"),
    (G, "airtable", "Airtable"),
    (G, "scaleai", "Scale AI"),
    (G, "samsara", "Samsara"),
    (G, "toast", "Toast"),
    (G, "squarespace", "Squarespace"),
    (G, "etsy", "Etsy"),
    (L, "palantir", "Palantir"),
    (L, "spotify", "Spotify"),
];

/// The built-in boards.
pub fn boards() -> Vec<Board> {
    STARTER
        .iter()
        .map(|(kind, token, company)| Board {
            kind: *kind,
            token: (*token).to_string(),
            company: (*company).to_string(),
        })
        .collect()
}

/// Whether the starter feed is on. It is on unless the person turned it off.
pub async fn is_enabled(db: &Db) -> Result<bool> {
    let row = sqlx::query("SELECT value FROM setting WHERE key = ?")
        .bind(SETTING)
        .fetch_optional(db.pool())
        .await
        .map_err(crate::db_err)?;
    Ok(row.is_none_or(|r| r.get::<String, _>("value") != "off"))
}

pub async fn set_enabled(db: &Db, on: bool) -> Result<()> {
    sqlx::query(
        "INSERT INTO setting (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(SETTING)
    .bind(if on { "on" } else { "off" })
    .execute(db.pool())
    .await
    .map_err(crate::db_err)?;
    Ok(())
}

fn same(a: &Board, b: &Board) -> bool {
    a.kind == b.kind && a.token.eq_ignore_ascii_case(&b.token)
}

/// Search the person's own boards and, unless turned off, the starter boards. A board the person
/// added is never searched twice. Problems with starter boards are logged, not reported.
pub async fn run_all(db: &Db, fetcher: &dyn Fetcher, now: &str) -> Result<SearchSummary> {
    let own = sources::load_boards(db).await?;
    let mut total = sources::run(db, fetcher, &own, now).await?;
    if is_enabled(db).await? {
        let extra: Vec<Board> = boards()
            .into_iter()
            .filter(|s| !own.iter().any(|o| same(o, s)))
            .collect();
        let s = sources::run(db, fetcher, &extra, now).await?;
        total.boards_ok += s.boards_ok;
        total.new += s.new;
        total.duplicates += s.duplicates;
        total.blocked += s.blocked;
        for f in &s.failures {
            tracing::info!("starter board skipped: {f}");
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests;
