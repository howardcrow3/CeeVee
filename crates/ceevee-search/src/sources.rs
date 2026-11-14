//! Job sources: public job-board feeds from Greenhouse and Lever.
//!
//! Parsing is pure so it can be tested on saved feeds. Fetching goes through the [`Fetcher`]
//! trait so tests never touch the network; [`HttpFetcher`] is the real one. Text from a feed is
//! written by someone else, so it is stored as data and never run as instructions.

use crate::{categories, postings};
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use postings::{Ingest, NewPosting};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardKind {
    Greenhouse,
    Lever,
}

/// One company's public board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    pub kind: BoardKind,
    /// The board token in the board's URL, such as `acme`.
    pub token: String,
    /// The name to store the company under.
    pub company: String,
}

impl Board {
    pub fn feed_url(&self) -> String {
        let t = &self.token;
        match self.kind {
            BoardKind::Greenhouse => {
                format!("https://boards-api.greenhouse.io/v1/boards/{t}/jobs?content=true")
            }
            BoardKind::Lever => format!("https://api.lever.co/v0/postings/{t}?mode=json"),
        }
    }
    fn source(&self) -> &'static str {
        match self.kind {
            BoardKind::Greenhouse => "greenhouse",
            BoardKind::Lever => "lever",
        }
    }
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// Turn the HTML Greenhouse sends (entity-escaped) into plain text.
pub fn html_to_text(html: &str) -> String {
    let unescaped = html
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&");
    let mut out = String::with_capacity(unescaped.len());
    let mut in_tag = false;
    for c in unescaped.chars() {
        match c {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Read a board's JSON feed into postings. Entries without a title or link are skipped.
pub fn parse_feed(board: &Board, json: &str) -> Result<Vec<NewPosting>> {
    let v: Value =
        serde_json::from_str(json).map_err(|e| Error::Invalid(format!("job feed: {e}")))?;
    let mut out = Vec::new();
    match board.kind {
        BoardKind::Greenhouse => {
            let jobs = v
                .get("jobs")
                .and_then(Value::as_array)
                .ok_or_else(|| Error::Invalid("Greenhouse feed has no jobs list".into()))?;
            for j in jobs {
                let (Some(title), Some(url)) = (text(j, "title"), text(j, "absolute_url")) else {
                    continue;
                };
                out.push(NewPosting {
                    source: board.source().into(),
                    url,
                    company: board.company.clone(),
                    title,
                    location: j.get("location").and_then(|l| text(l, "name")),
                    body: text(j, "content").map(|c| html_to_text(&c)),
                    ..NewPosting::default()
                });
            }
        }
        BoardKind::Lever => {
            let jobs = v
                .as_array()
                .ok_or_else(|| Error::Invalid("Lever feed is not a list".into()))?;
            for j in jobs {
                let (Some(title), Some(url)) = (text(j, "text"), text(j, "hostedUrl")) else {
                    continue;
                };
                out.push(NewPosting {
                    source: board.source().into(),
                    url,
                    company: board.company.clone(),
                    title,
                    location: j.get("categories").and_then(|c| text(c, "location")),
                    body: text(j, "descriptionPlain"),
                    ..NewPosting::default()
                });
            }
        }
    }
    Ok(out)
}

pub type FetchFuture = Pin<Box<dyn Future<Output = Result<String>> + Send>>;

pub trait Fetcher: Send + Sync {
    fn get(&self, url: String) -> FetchFuture;
}

/// Fetches over HTTPS with a short timeout.
pub struct HttpFetcher {
    client: reqwest::Client,
}

impl HttpFetcher {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("CeeVee job search")
            .build()
            .map_err(|e| Error::Other(e.to_string()))?;
        Ok(Self { client })
    }
}

impl Fetcher for HttpFetcher {
    fn get(&self, url: String) -> FetchFuture {
        let client = self.client.clone();
        Box::pin(async move {
            let res = client
                .get(&url)
                .send()
                .await
                .map_err(|e| Error::Other(format!("fetching {url}: {e}")))?;
            if !res.status().is_success() {
                return Err(Error::Other(format!("{url} answered {}", res.status())));
            }
            res.text()
                .await
                .map_err(|e| Error::Other(format!("reading {url}: {e}")))
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchSummary {
    pub boards_ok: u32,
    pub new: u32,
    pub duplicates: u32,
    pub blocked: u32,
    /// One line per board that could not be read, so the user can see what failed.
    pub failures: Vec<String>,
}

/// Read every board, store new postings and score them against the active categories. One bad
/// board never stops the others.
pub async fn run(
    db: &Db,
    fetcher: &dyn Fetcher,
    boards: &[Board],
    now: &str,
) -> Result<SearchSummary> {
    let mut s = SearchSummary::default();
    for b in boards {
        let parsed = match fetcher.get(b.feed_url()).await {
            Ok(body) => parse_feed(b, &body),
            Err(e) => Err(e),
        };
        let items = match parsed {
            Ok(i) => i,
            Err(e) => {
                s.failures.push(format!("{}: {e}", b.company));
                continue;
            }
        };
        s.boards_ok += 1;
        for p in items {
            match postings::ingest(db, &p, now).await {
                Ok(Ingest::New(id)) => {
                    s.new += 1;
                    categories::score_posting(db, id).await?;
                }
                Ok(Ingest::Duplicate(_)) => s.duplicates += 1,
                Ok(Ingest::Blocked) => s.blocked += 1,
                Err(e) => s.failures.push(format!("{}: {e}", b.company)),
            }
        }
    }
    Ok(s)
}

/// The current time as ISO-8601 UTC, such as `2026-11-13T09:46:00Z`.
pub fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    iso_from_secs(secs)
}

pub(crate) fn iso_from_secs(secs: u64) -> String {
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

const BOARDS_KEY: &str = "search.boards";

/// The boards the user chose to watch, from the settings table.
pub async fn load_boards(db: &Db) -> Result<Vec<Board>> {
    let row = sqlx::query("SELECT value FROM setting WHERE key = ?")
        .bind(BOARDS_KEY)
        .fetch_optional(db.pool())
        .await
        .map_err(crate::db_err)?;
    match row {
        None => Ok(Vec::new()),
        Some(r) => serde_json::from_str(&r.get::<String, _>("value"))
            .map_err(|e| Error::Invalid(format!("saved boards: {e}"))),
    }
}

/// Save the watched boards. A board needs a token and a company name; tokens may hold only
/// letters, digits, dashes and underscores so they cannot change the feed URL.
pub async fn save_boards(db: &Db, boards: &[Board]) -> Result<()> {
    for b in boards {
        let ok = !b.token.is_empty()
            && b.token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !ok || b.company.trim().is_empty() {
            return Err(Error::Invalid(format!(
                "board \"{}\" needs a company name and a token of letters, digits and dashes",
                b.company
            )));
        }
    }
    let json = serde_json::to_string(boards).map_err(|e| Error::Other(e.to_string()))?;
    sqlx::query(
        "INSERT INTO setting (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(BOARDS_KEY)
    .bind(json)
    .execute(db.pool())
    .await
    .map_err(crate::db_err)?;
    Ok(())
}

#[cfg(test)]
mod tests;
