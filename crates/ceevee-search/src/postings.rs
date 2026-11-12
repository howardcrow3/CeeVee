//! Postings: ingesting what the sources found, without duplicates.
//!
//! The same job often appears on several boards and at several URLs. A posting counts as a
//! duplicate when its URL is known, or when the same company already has a posting with the same
//! title and location (compared as plain words).

use crate::{companies, db_err, words};
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Default)]
pub struct NewPosting {
    pub source: String,
    pub url: String,
    pub company: String,
    pub title: String,
    pub location: Option<String>,
    pub level: Option<String>,
    pub pay: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ingest {
    /// A new posting was stored.
    New(i64),
    /// Already known (same URL, or same job from another source); only `last_seen` moved.
    Duplicate(i64),
    /// The company is blocked, so nothing was stored.
    Blocked,
}

/// Store a posting unless it is a duplicate or from a blocked company. `now` is ISO-8601 UTC.
pub async fn ingest(db: &Db, p: &NewPosting, now: &str) -> Result<Ingest> {
    if p.url.trim().is_empty() || p.title.trim().is_empty() {
        return Err(Error::Invalid("a posting needs a URL and a title".into()));
    }
    if let Some(id) = sqlx::query("SELECT id FROM posting WHERE url = ?")
        .bind(&p.url)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .map(|r| r.get::<i64, _>("id"))
    {
        touch(db, id, now).await?;
        return Ok(Ingest::Duplicate(id));
    }
    let company = companies::find_or_create(db, &p.company).await?;
    if company.status == companies::Status::Blocked {
        return Ok(Ingest::Blocked);
    }
    let title = words(&p.title);
    let location = words(p.location.as_deref().unwrap_or(""));
    let same = sqlx::query("SELECT id, title, location FROM posting WHERE company_id = ?")
        .bind(company.id)
        .fetch_all(db.pool())
        .await
        .map_err(db_err)?;
    for r in &same {
        let t: String = r.get("title");
        let l: Option<String> = r.get("location");
        if words(&t) == title && words(l.as_deref().unwrap_or("")) == location {
            let id: i64 = r.get("id");
            touch(db, id, now).await?;
            return Ok(Ingest::Duplicate(id));
        }
    }
    let id = sqlx::query(
        "INSERT INTO posting (source, url, company_id, title, location, level, pay, body, first_seen, last_seen) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&p.source)
    .bind(&p.url)
    .bind(company.id)
    .bind(p.title.trim())
    .bind(&p.location)
    .bind(&p.level)
    .bind(&p.pay)
    .bind(&p.body)
    .bind(now)
    .bind(now)
    .execute(db.pool())
    .await
    .map_err(db_err)?
    .last_insert_rowid();
    Ok(Ingest::New(id))
}

async fn touch(db: &Db, id: i64, now: &str) -> Result<()> {
    sqlx::query("UPDATE posting SET last_seen = ? WHERE id = ?")
        .bind(now)
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    New,
    Saved,
    Dismissed,
}

impl State {
    fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Saved => "saved",
            Self::Dismissed => "dismissed",
        }
    }
}

pub async fn set_state(db: &Db, id: i64, state: State) -> Result<()> {
    let n = sqlx::query("UPDATE posting SET state = ? WHERE id = ?")
        .bind(state.as_str())
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?
        .rows_affected();
    if n == 0 {
        return Err(Error::NotFound(format!("posting {id}")));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Posting {
    pub id: i64,
    pub source: String,
    pub url: String,
    pub company_id: Option<i64>,
    pub title: String,
    pub location: Option<String>,
    pub level: Option<String>,
    pub pay: Option<String>,
    pub body: Option<String>,
    pub score: Option<f64>,
    pub first_seen: String,
}

pub async fn get(db: &Db, id: i64) -> Result<Posting> {
    let r = sqlx::query(
        "SELECT id, source, url, company_id, title, location, level, pay, body, score, first_seen \
         FROM posting WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db.pool())
    .await
    .map_err(db_err)?
    .ok_or_else(|| Error::NotFound(format!("posting {id}")))?;
    Ok(Posting {
        id: r.get("id"),
        source: r.get("source"),
        url: r.get("url"),
        company_id: r.get("company_id"),
        title: r.get("title"),
        location: r.get("location"),
        level: r.get("level"),
        pay: r.get("pay"),
        body: r.get("body"),
        score: r.get("score"),
        first_seen: r.get("first_seen"),
    })
}

/// New (not dismissed) postings for a category, best score first.
pub async fn for_category(db: &Db, category_id: i64, min_score: f64) -> Result<Vec<Posting>> {
    let rows = sqlx::query(
        "SELECT p.id FROM posting p JOIN posting_category pc ON pc.posting_id = p.id \
         WHERE pc.category_id = ? AND pc.score >= ? AND p.state != 'dismissed' \
         ORDER BY pc.score DESC, p.id",
    )
    .bind(category_id)
    .bind(min_score)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(get(db, r.get("id")).await?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
