//! The record of what was prepared or sent for a job: the form answers, which resume and letter
//! went with them, and the posting text as it was that day. Written when a form is filled and again
//! when the person submits, and read back from the job's detail pane ("Open record").

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;

fn db_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Prepared,
    Submitted,
}

impl Event {
    fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Submitted => "submitted",
        }
    }
}

/// What to save. `filled` and `skipped` are JSON arrays shaped like the apply tool's output.
#[derive(Debug, Clone)]
pub struct NewRecord {
    pub posting_id: i64,
    pub application_id: Option<i64>,
    pub event: Event,
    pub url: String,
    pub ats: Option<String>,
    pub resume_version_id: Option<i64>,
    pub cover_letter_version_id: Option<i64>,
    pub filled: Value,
    pub skipped: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: i64,
    pub posting_id: Option<i64>,
    pub application_id: Option<i64>,
    pub event: String,
    pub url: String,
    pub ats: Option<String>,
    pub job_title: Option<String>,
    pub posting_text: Option<String>,
    pub resume_version_id: Option<i64>,
    pub cover_letter_version_id: Option<i64>,
    pub filled: Value,
    pub skipped: Value,
    pub created_at: String,
}

/// Save a record. The job title and posting text are copied from the posting at this moment.
pub async fn save(db: &Db, r: &NewRecord) -> Result<i64> {
    if !r.filled.is_array() || !r.skipped.is_array() {
        return Err(Error::Invalid("filled and skipped must be lists".into()));
    }
    let posting = sqlx::query("SELECT title, body FROM posting WHERE id = ?")
        .bind(r.posting_id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .ok_or_else(|| Error::NotFound(format!("posting {}", r.posting_id)))?;
    let title: String = posting.get("title");
    let body: Option<String> = posting.get("body");
    let id = sqlx::query(
        "INSERT INTO application_record (posting_id, application_id, event, url, ats, job_title, \
         posting_text, resume_version_id, cover_letter_version_id, filled, skipped) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(r.posting_id)
    .bind(r.application_id)
    .bind(r.event.as_str())
    .bind(&r.url)
    .bind(&r.ats)
    .bind(title)
    .bind(body)
    .bind(r.resume_version_id)
    .bind(r.cover_letter_version_id)
    .bind(r.filled.to_string())
    .bind(r.skipped.to_string())
    .execute(db.pool())
    .await
    .map_err(db_err)?
    .last_insert_rowid();
    Ok(id)
}

/// Records for a job, newest first.
pub async fn list_for_posting(db: &Db, posting_id: i64) -> Result<Vec<Record>> {
    let rows = sqlx::query(
        "SELECT * FROM application_record WHERE posting_id = ? ORDER BY created_at DESC, id DESC",
    )
    .bind(posting_id)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    rows.iter().map(from_row).collect()
}

fn parse(s: String) -> Result<Value> {
    serde_json::from_str(&s).map_err(|e| Error::Database(e.to_string()))
}

fn from_row(r: &sqlx::sqlite::SqliteRow) -> Result<Record> {
    Ok(Record {
        id: r.get("id"),
        posting_id: r.get("posting_id"),
        application_id: r.get("application_id"),
        event: r.get("event"),
        url: r.get("url"),
        ats: r.get("ats"),
        job_title: r.get("job_title"),
        posting_text: r.get("posting_text"),
        resume_version_id: r.get("resume_version_id"),
        cover_letter_version_id: r.get("cover_letter_version_id"),
        filled: parse(r.get("filled"))?,
        skipped: parse(r.get("skipped"))?,
        created_at: r.get("created_at"),
    })
}

#[cfg(test)]
mod tests;
