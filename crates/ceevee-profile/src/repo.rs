//! Reading and writing the profile tables.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde::{Deserialize, Serialize};
use sqlx::Row;

fn db_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Basics {
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Experience,
    Education,
    Project,
    Certification,
    Publication,
    Reference,
}

impl EntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Experience => "experience",
            Self::Education => "education",
            Self::Project => "project",
            Self::Certification => "certification",
            Self::Publication => "publication",
            Self::Reference => "reference",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "experience" => Self::Experience,
            "education" => Self::Education,
            "project" => Self::Project,
            "certification" => Self::Certification,
            "publication" => Self::Publication,
            "reference" => Self::Reference,
            other => return Err(Error::Database(format!("unknown entry kind {other}"))),
        })
    }
}

/// One line of the resume: a job, a degree, a project and so on. A missing end date on an
/// experience means the person still works there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: Option<i64>,
    pub kind: EntryKind,
    pub title: String,
    pub organization: Option<String>,
    pub location: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub bullets: Vec<String>,
}

impl Entry {
    /// Two entries describe the same thing when kind, organization, title and start date agree,
    /// ignoring case and surrounding space. Used to avoid duplicates on import.
    pub fn same_as(&self, other: &Entry) -> bool {
        fn n(s: &Option<String>) -> String {
            s.as_deref().unwrap_or("").trim().to_lowercase()
        }
        self.kind == other.kind
            && self.title.trim().to_lowercase() == other.title.trim().to_lowercase()
            && n(&self.organization) == n(&other.organization)
            && n(&self.start_date) == n(&other.start_date)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    pub question_key: String,
    pub answer: String,
    /// EEO, salary and similar answers are filled only when the user has saved them for that purpose.
    pub sensitive: bool,
}

/// A change to the profile that the user can accept or reject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "change", rename_all = "snake_case")]
pub enum Change {
    SetBasic {
        field: String,
        old: Option<String>,
        new: String,
    },
    AddEntry {
        entry: Entry,
    },
    AddSkill {
        name: String,
    },
}

pub async fn get_basics(db: &Db) -> Result<Basics> {
    let row =
        sqlx::query("SELECT full_name, email, phone, location, summary FROM profile WHERE id = 1")
            .fetch_optional(db.pool())
            .await
            .map_err(db_err)?;
    Ok(row.map_or_else(Basics::default, |r| Basics {
        full_name: r.get("full_name"),
        email: r.get("email"),
        phone: r.get("phone"),
        location: r.get("location"),
        summary: r.get("summary"),
    }))
}

pub async fn set_basics(db: &Db, b: &Basics) -> Result<()> {
    sqlx::query(
        "INSERT INTO profile(id, full_name, email, phone, location, summary) VALUES (1, ?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET full_name = excluded.full_name, email = excluded.email, \
         phone = excluded.phone, location = excluded.location, summary = excluded.summary, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
    )
    .bind(&b.full_name)
    .bind(&b.email)
    .bind(&b.phone)
    .bind(&b.location)
    .bind(&b.summary)
    .execute(db.pool())
    .await
    .map_err(db_err)?;
    Ok(())
}

fn entry_from(r: &sqlx::sqlite::SqliteRow) -> Result<Entry> {
    let bullets: String = r.get("bullets");
    Ok(Entry {
        id: Some(r.get("id")),
        kind: EntryKind::parse(&r.get::<String, _>("kind"))?,
        title: r.get("title"),
        organization: r.get("organization"),
        location: r.get("location"),
        start_date: r.get("start_date"),
        end_date: r.get("end_date"),
        bullets: serde_json::from_str(&bullets)
            .map_err(|e| Error::Database(format!("bullets: {e}")))?,
    })
}

/// Entries of one kind, newest start date first (entries with no date last), then by order added.
pub async fn list_entries(db: &Db, kind: EntryKind) -> Result<Vec<Entry>> {
    let rows = sqlx::query(
        "SELECT * FROM profile_entry WHERE kind = ? \
         ORDER BY (start_date IS NULL), start_date DESC, sort_order, id",
    )
    .bind(kind.as_str())
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    rows.iter().map(entry_from).collect()
}

pub async fn add_entry(db: &Db, e: &Entry) -> Result<i64> {
    let order: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM profile_entry WHERE kind = ?",
    )
    .bind(e.kind.as_str())
    .fetch_one(db.pool())
    .await
    .map_err(db_err)?;
    sqlx::query_scalar(
        "INSERT INTO profile_entry(kind, title, organization, location, start_date, end_date, bullets, sort_order) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(e.kind.as_str())
    .bind(&e.title)
    .bind(&e.organization)
    .bind(&e.location)
    .bind(&e.start_date)
    .bind(&e.end_date)
    .bind(serde_json::to_string(&e.bullets).unwrap_or_else(|_| "[]".into()))
    .bind(order)
    .fetch_one(db.pool())
    .await
    .map_err(db_err)
}

pub async fn delete_entry(db: &Db, id: i64) -> Result<()> {
    let n = sqlx::query("DELETE FROM profile_entry WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?
        .rows_affected();
    if n == 0 {
        return Err(Error::NotFound(format!("profile entry {id}")));
    }
    Ok(())
}

pub async fn list_skills(db: &Db) -> Result<Vec<String>> {
    sqlx::query_scalar("SELECT name FROM skill ORDER BY sort_order, id")
        .fetch_all(db.pool())
        .await
        .map_err(db_err)
}

/// Add a skill unless one with the same name (ignoring case) exists. Returns whether it was added.
pub async fn add_skill(db: &Db, name: &str) -> Result<bool> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("skill name is empty".into()));
    }
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skill WHERE lower(name) = lower(?)")
        .bind(name)
        .fetch_one(db.pool())
        .await
        .map_err(db_err)?;
    if exists > 0 {
        return Ok(false);
    }
    sqlx::query("INSERT INTO skill(name, sort_order) VALUES (?, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM skill))")
        .bind(name)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(true)
}

pub async fn set_fact(db: &Db, f: &Fact) -> Result<()> {
    sqlx::query(
        "INSERT INTO fact(question_key, answer, sensitive) VALUES (?, ?, ?) \
         ON CONFLICT(question_key) DO UPDATE SET answer = excluded.answer, sensitive = excluded.sensitive, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
    )
    .bind(&f.question_key)
    .bind(&f.answer)
    .bind(f.sensitive)
    .execute(db.pool())
    .await
    .map_err(db_err)?;
    Ok(())
}

pub async fn get_fact(db: &Db, key: &str) -> Result<Option<Fact>> {
    let row =
        sqlx::query("SELECT question_key, answer, sensitive FROM fact WHERE question_key = ?")
            .bind(key)
            .fetch_optional(db.pool())
            .await
            .map_err(db_err)?;
    Ok(row.map(|r| Fact {
        question_key: r.get("question_key"),
        answer: r.get("answer"),
        sensitive: r.get::<i64, _>("sensitive") != 0,
    }))
}

/// Apply one accepted change. Adding something that already exists is a no-op, so applying the same
/// change twice is safe.
pub async fn apply(db: &Db, change: &Change) -> Result<()> {
    match change {
        Change::SetBasic { field, new, .. } => {
            let mut b = get_basics(db).await?;
            let v = Some(new.clone());
            match field.as_str() {
                "full_name" => b.full_name = new.clone(),
                "email" => b.email = v,
                "phone" => b.phone = v,
                "location" => b.location = v,
                "summary" => b.summary = v,
                other => return Err(Error::Invalid(format!("unknown profile field {other}"))),
            }
            set_basics(db, &b).await
        }
        Change::AddEntry { entry } => {
            let existing = list_entries(db, entry.kind).await?;
            if existing.iter().any(|e| e.same_as(entry)) {
                return Ok(());
            }
            add_entry(db, entry).await.map(|_| ())
        }
        Change::AddSkill { name } => add_skill(db, name).await.map(|_| ()),
    }
}

#[cfg(test)]
mod tests;
