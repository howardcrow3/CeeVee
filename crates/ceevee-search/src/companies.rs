//! Companies: found by name or alias, never duplicated by punctuation or "Inc".

use crate::{db_err, words};
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Normal,
    Watchlist,
    Blocked,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Watchlist => "watchlist",
            Self::Blocked => "blocked",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "watchlist" => Self::Watchlist,
            "blocked" => Self::Blocked,
            _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Company {
    pub id: i64,
    pub name: String,
    pub aliases: Vec<String>,
    pub status: Status,
}

const SUFFIXES: &[&str] = &[
    "inc",
    "incorporated",
    "llc",
    "ltd",
    "limited",
    "corp",
    "corporation",
    "co",
    "company",
    "gmbh",
    "plc",
];

/// The name with punctuation and legal suffixes removed, for matching.
pub fn normalize_name(name: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let w = words(name);
    parts.extend(w.split(' ').filter(|p| !p.is_empty()));
    while parts.len() > 1 && SUFFIXES.contains(parts.last().unwrap()) {
        parts.pop();
    }
    parts.join(" ")
}

fn from_row(r: &sqlx::sqlite::SqliteRow) -> Result<Company> {
    let aliases: String = r.get("aliases");
    Ok(Company {
        id: r.get("id"),
        name: r.get("name"),
        aliases: serde_json::from_str(&aliases).map_err(|e| Error::Other(e.to_string()))?,
        status: Status::parse(r.get::<String, _>("status").as_str()),
    })
}

pub async fn list(db: &Db) -> Result<Vec<Company>> {
    let rows = sqlx::query("SELECT id, name, aliases, status FROM company ORDER BY name")
        .fetch_all(db.pool())
        .await
        .map_err(db_err)?;
    rows.iter().map(from_row).collect()
}

pub async fn get(db: &Db, id: i64) -> Result<Company> {
    let row = sqlx::query("SELECT id, name, aliases, status FROM company WHERE id = ?")
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .ok_or_else(|| Error::NotFound(format!("company {id}")))?;
    from_row(&row)
}

/// Find a company whose name or alias matches, or create it.
pub async fn find_or_create(db: &Db, name: &str) -> Result<Company> {
    let key = normalize_name(name);
    if key.is_empty() {
        return Err(Error::Invalid("company name is empty".into()));
    }
    for c in list(db).await? {
        if normalize_name(&c.name) == key || c.aliases.iter().any(|a| normalize_name(a) == key) {
            return Ok(c);
        }
    }
    let id = sqlx::query("INSERT INTO company (name) VALUES (?)")
        .bind(name.trim())
        .execute(db.pool())
        .await
        .map_err(db_err)?
        .last_insert_rowid();
    get(db, id).await
}

pub async fn add_alias(db: &Db, id: i64, alias: &str) -> Result<()> {
    let mut c = get(db, id).await?;
    if !c
        .aliases
        .iter()
        .any(|a| normalize_name(a) == normalize_name(alias))
    {
        c.aliases.push(alias.trim().to_string());
        sqlx::query("UPDATE company SET aliases = ? WHERE id = ?")
            .bind(serde_json::to_string(&c.aliases).map_err(|e| Error::Other(e.to_string()))?)
            .bind(id)
            .execute(db.pool())
            .await
            .map_err(db_err)?;
    }
    Ok(())
}

pub async fn set_status(db: &Db, id: i64, status: Status) -> Result<()> {
    get(db, id).await?;
    sqlx::query("UPDATE company SET status = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}
