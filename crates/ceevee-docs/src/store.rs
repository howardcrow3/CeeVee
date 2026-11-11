//! The document store. Every change is a new version, so nothing is lost and any version can be
//! restored; "undo" just moves the document back to the version before the current one.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use sqlx::Row;

fn db_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    BaseResume,
    ResumeVariant,
    TailoredResume,
    CoverLetter,
    Email,
}

impl DocKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BaseResume => "base_resume",
            Self::ResumeVariant => "resume_variant",
            Self::TailoredResume => "tailored_resume",
            Self::CoverLetter => "cover_letter",
            Self::Email => "email",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "base_resume" => Self::BaseResume,
            "resume_variant" => Self::ResumeVariant,
            "tailored_resume" => Self::TailoredResume,
            "cover_letter" => Self::CoverLetter,
            "email" => Self::Email,
            other => return Err(Error::Database(format!("unknown document kind {other}"))),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub id: i64,
    pub kind: DocKind,
    pub title: String,
    pub application_id: Option<i64>,
    pub category_id: Option<i64>,
    pub current_version_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub id: i64,
    pub document_id: i64,
    /// Resume JSON or letter Markdown.
    pub content: String,
    /// Why this version exists: "created", "edited by you", "tailored", "chat: make it shorter".
    pub cause: String,
    /// The task and prompt version that wrote it, when an AI did.
    pub created_with: Option<String>,
    pub created_at: String,
}

fn doc_from(r: &sqlx::sqlite::SqliteRow) -> Result<Document> {
    Ok(Document {
        id: r.get("id"),
        kind: DocKind::parse(&r.get::<String, _>("kind"))?,
        title: r.get("title"),
        application_id: r.get("application_id"),
        category_id: r.get("category_id"),
        current_version_id: r.get("current_version_id"),
    })
}

fn version_from(r: &sqlx::sqlite::SqliteRow) -> Version {
    Version {
        id: r.get("id"),
        document_id: r.get("document_id"),
        content: r.get("content"),
        cause: r.get("cause"),
        created_with: r.get("created_with"),
        created_at: r.get("created_at"),
    }
}

/// Create a document with its first version. Returns the document id.
pub async fn create(
    db: &Db,
    kind: DocKind,
    title: &str,
    application_id: Option<i64>,
    category_id: Option<i64>,
    content: &str,
    cause: &str,
) -> Result<i64> {
    let mut tx = db.pool().begin().await.map_err(db_err)?;
    let doc_id: i64 = sqlx::query_scalar(
        "INSERT INTO document(kind, title, application_id, category_id) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(kind.as_str())
    .bind(title)
    .bind(application_id)
    .bind(category_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;
    let version_id: i64 = sqlx::query_scalar(
        "INSERT INTO document_version(document_id, content, cause) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(doc_id)
    .bind(content)
    .bind(cause)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;
    sqlx::query("UPDATE document SET current_version_id = ? WHERE id = ?")
        .bind(version_id)
        .bind(doc_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(doc_id)
}

pub async fn get(db: &Db, id: i64) -> Result<Document> {
    let row = sqlx::query("SELECT * FROM document WHERE id = ?")
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .ok_or_else(|| Error::NotFound(format!("document {id}")))?;
    doc_from(&row)
}

/// Documents, newest first, optionally of one kind or for one application.
pub async fn list(
    db: &Db,
    kind: Option<DocKind>,
    application_id: Option<i64>,
) -> Result<Vec<Document>> {
    let rows = sqlx::query(
        "SELECT * FROM document WHERE (?1 IS NULL OR kind = ?1) AND (?2 IS NULL OR application_id = ?2) \
         ORDER BY id DESC",
    )
    .bind(kind.map(DocKind::as_str))
    .bind(application_id)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    rows.iter().map(doc_from).collect()
}

pub async fn rename(db: &Db, id: i64, title: &str) -> Result<()> {
    let n = sqlx::query("UPDATE document SET title = ? WHERE id = ?")
        .bind(title)
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?
        .rows_affected();
    if n == 0 {
        return Err(Error::NotFound(format!("document {id}")));
    }
    Ok(())
}

pub async fn delete(db: &Db, id: i64) -> Result<()> {
    let n = sqlx::query("DELETE FROM document WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?
        .rows_affected();
    if n == 0 {
        return Err(Error::NotFound(format!("document {id}")));
    }
    Ok(())
}

/// The version the document is currently on.
pub async fn current(db: &Db, doc_id: i64) -> Result<Version> {
    let row = sqlx::query(
        "SELECT v.* FROM document d JOIN document_version v ON v.id = d.current_version_id WHERE d.id = ?",
    )
    .bind(doc_id)
    .fetch_optional(db.pool())
    .await
    .map_err(db_err)?
    .ok_or_else(|| Error::NotFound(format!("document {doc_id}")))?;
    Ok(version_from(&row))
}

/// All versions, newest first.
pub async fn versions(db: &Db, doc_id: i64) -> Result<Vec<Version>> {
    let rows = sqlx::query("SELECT * FROM document_version WHERE document_id = ? ORDER BY id DESC")
        .bind(doc_id)
        .fetch_all(db.pool())
        .await
        .map_err(db_err)?;
    Ok(rows.iter().map(version_from).collect())
}

/// Add a version and make it current. If the content equals the current version nothing is added
/// and the current version's id is returned, so saving an unchanged editor does not clutter history.
pub async fn add_version(
    db: &Db,
    doc_id: i64,
    content: &str,
    cause: &str,
    created_with: Option<&str>,
) -> Result<i64> {
    let cur = current(db, doc_id).await?;
    if cur.content == content {
        return Ok(cur.id);
    }
    let mut tx = db.pool().begin().await.map_err(db_err)?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO document_version(document_id, content, cause, created_with) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(doc_id)
    .bind(content)
    .bind(cause)
    .bind(created_with)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;
    sqlx::query("UPDATE document SET current_version_id = ? WHERE id = ?")
        .bind(id)
        .bind(doc_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(id)
}

/// Make an earlier (or later) version current again. History is kept as it is.
pub async fn restore(db: &Db, doc_id: i64, version_id: i64) -> Result<Version> {
    let row = sqlx::query("SELECT * FROM document_version WHERE id = ? AND document_id = ?")
        .bind(version_id)
        .bind(doc_id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .ok_or_else(|| Error::NotFound(format!("version {version_id} of document {doc_id}")))?;
    sqlx::query("UPDATE document SET current_version_id = ? WHERE id = ?")
        .bind(version_id)
        .bind(doc_id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(version_from(&row))
}

/// Step back to the version before the current one. Fails at the oldest version.
pub async fn undo(db: &Db, doc_id: i64) -> Result<Version> {
    let cur = current(db, doc_id).await?;
    let prev: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM document_version WHERE document_id = ? AND id < ? ORDER BY id DESC LIMIT 1",
    )
    .bind(doc_id)
    .bind(cur.id)
    .fetch_optional(db.pool())
    .await
    .map_err(db_err)?;
    let prev = prev.ok_or_else(|| Error::Invalid("this is the oldest version".into()))?;
    restore(db, doc_id, prev).await
}

/// Step forward again after an undo. Fails at the newest version.
pub async fn redo(db: &Db, doc_id: i64) -> Result<Version> {
    let cur = current(db, doc_id).await?;
    let next: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM document_version WHERE document_id = ? AND id > ? ORDER BY id LIMIT 1",
    )
    .bind(doc_id)
    .bind(cur.id)
    .fetch_optional(db.pool())
    .await
    .map_err(db_err)?;
    let next = next.ok_or_else(|| Error::Invalid("this is the newest version".into()))?;
    restore(db, doc_id, next).await
}

#[cfg(test)]
mod tests;
