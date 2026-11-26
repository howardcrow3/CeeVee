//! The people list: add, edit, find and merge contacts, and keep a timeline of touches with each.
//! Everything stays in the local database.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde::{Deserialize, Serialize};
use sqlx::Row;

fn db_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Close,
    Warm,
    Weak,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::Warm => "warm",
            Self::Weak => "weak",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "close" => Self::Close,
            "weak" => Self::Weak,
            _ => Self::Warm,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Note,
    Email,
    Meeting,
    Call,
    Message,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Email => "email",
            Self::Meeting => "meeting",
            Self::Call => "call",
            Self::Message => "message",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "email" => Self::Email,
            "meeting" => Self::Meeting,
            "call" => Self::Call,
            "message" => Self::Message,
            _ => Self::Note,
        }
    }
}

/// What to save when adding or editing a person.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NewPerson {
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub profile_url: Option<String>,
    pub how_met: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub company: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: i64,
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub profile_url: Option<String>,
    pub how_met: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub tier: Tier,
    pub source: String,
    pub last_contacted: Option<String>,
    /// Where they work now, if known.
    pub company: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Touch {
    pub id: i64,
    pub kind: Kind,
    pub summary: String,
    pub at: String,
}

fn clean(s: &Option<String>) -> Option<String> {
    s.as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

fn clean_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tags {
        let t = t.trim().to_lowercase();
        if !t.is_empty() && !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

const SELECT: &str =
    "SELECT c.id, c.full_name, c.email, c.phone, c.profile_url, c.how_met, c.notes, \
     c.tags, c.tier, c.source, c.last_contacted, \
     (SELECT co.name FROM contact_role r JOIN company co ON co.id = r.company_id \
        WHERE r.contact_id = c.id AND r.is_current = 1 ORDER BY r.id DESC LIMIT 1) AS company, \
     (SELECT r.title FROM contact_role r WHERE r.contact_id = c.id AND r.is_current = 1 \
        ORDER BY r.id DESC LIMIT 1) AS title \
     FROM contact c";

fn person(r: &sqlx::sqlite::SqliteRow) -> Person {
    let tags: String = r.get("tags");
    Person {
        id: r.get("id"),
        full_name: r.get("full_name"),
        email: r.get("email"),
        phone: r.get("phone"),
        profile_url: r.get("profile_url"),
        how_met: r.get("how_met"),
        notes: r.get("notes"),
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        tier: Tier::parse(&r.get::<String, _>("tier")),
        source: r.get("source"),
        last_contacted: r.get("last_contacted"),
        company: r.get("company"),
        title: r.get("title"),
    }
}

async fn set_role(
    db: &Db,
    id: i64,
    company: &Option<String>,
    title: &Option<String>,
) -> Result<()> {
    if let Some(name) = clean(company) {
        let co = ceevee_search::companies::find_or_create(db, &name).await?;
        sqlx::query("DELETE FROM contact_role WHERE contact_id = ? AND is_current = 1")
            .bind(id)
            .execute(db.pool())
            .await
            .map_err(db_err)?;
        sqlx::query(
            "INSERT INTO contact_role(contact_id, company_id, title, is_current) VALUES (?, ?, ?, 1)",
        )
        .bind(id)
        .bind(co.id)
        .bind(clean(title))
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    }
    Ok(())
}

/// Add a person by hand. Needs a name.
pub async fn add(db: &Db, p: &NewPerson, source: &str) -> Result<i64> {
    let name = p.full_name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("a person needs a name".into()));
    }
    let tags =
        serde_json::to_string(&clean_tags(&p.tags)).map_err(|e| Error::Other(e.to_string()))?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO contact(full_name, email, phone, profile_url, how_met, notes, tags, source) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(name)
    .bind(clean(&p.email))
    .bind(clean(&p.phone))
    .bind(clean(&p.profile_url))
    .bind(clean(&p.how_met))
    .bind(clean(&p.notes))
    .bind(tags)
    .bind(source)
    .fetch_one(db.pool())
    .await
    .map_err(db_err)?;
    set_role(db, id, &p.company, &p.title).await?;
    Ok(id)
}

/// Replace a person's details. The company and title change only when a company is given.
pub async fn update(db: &Db, id: i64, p: &NewPerson) -> Result<()> {
    let name = p.full_name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("a person needs a name".into()));
    }
    let tags =
        serde_json::to_string(&clean_tags(&p.tags)).map_err(|e| Error::Other(e.to_string()))?;
    let n = sqlx::query(
        "UPDATE contact SET full_name = ?, email = ?, phone = ?, profile_url = ?, how_met = ?, \
         notes = ?, tags = ? WHERE id = ?",
    )
    .bind(name)
    .bind(clean(&p.email))
    .bind(clean(&p.phone))
    .bind(clean(&p.profile_url))
    .bind(clean(&p.how_met))
    .bind(clean(&p.notes))
    .bind(tags)
    .bind(id)
    .execute(db.pool())
    .await
    .map_err(db_err)?
    .rows_affected();
    if n == 0 {
        return Err(Error::NotFound(format!("person {id}")));
    }
    set_role(db, id, &p.company, &p.title).await
}

pub async fn get(db: &Db, id: i64) -> Result<Person> {
    let row = sqlx::query(&format!("{SELECT} WHERE c.id = ?"))
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .ok_or_else(|| Error::NotFound(format!("person {id}")))?;
    Ok(person(&row))
}

/// Everyone, or those whose name, company, title, tags or how-met text contain `query`. Closest
/// first, then by name.
pub async fn list(db: &Db, query: Option<&str>, tier: Option<Tier>) -> Result<Vec<Person>> {
    let rows = sqlx::query(&format!(
        "{SELECT} ORDER BY CASE c.tier WHEN 'close' THEN 0 WHEN 'warm' THEN 1 ELSE 2 END, \
         c.full_name COLLATE NOCASE"
    ))
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    let q = query
        .map(|q| q.trim().to_lowercase())
        .filter(|q| !q.is_empty());
    Ok(rows
        .iter()
        .map(person)
        .filter(|p| tier.is_none_or(|t| p.tier == t))
        .filter(|p| match &q {
            None => true,
            Some(q) => {
                let hay = format!(
                    "{} {} {} {} {}",
                    p.full_name,
                    p.company.as_deref().unwrap_or(""),
                    p.title.as_deref().unwrap_or(""),
                    p.how_met.as_deref().unwrap_or(""),
                    p.tags.join(" ")
                )
                .to_lowercase();
                hay.contains(q.as_str())
            }
        })
        .collect())
}

pub async fn set_tier(db: &Db, id: i64, tier: Tier) -> Result<()> {
    let n = sqlx::query("UPDATE contact SET tier = ? WHERE id = ?")
        .bind(tier.as_str())
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

pub async fn delete(db: &Db, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM contact WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Write a note, email, meeting, call or message into a person's timeline and mark them as
/// contacted at that time (`at` is ISO-8601 UTC). Only the latest time counts for "last contacted".
pub async fn add_touch(db: &Db, id: i64, kind: Kind, summary: &str, at: &str) -> Result<i64> {
    let s = summary.trim();
    if s.is_empty() {
        return Err(Error::Invalid("say what happened".into()));
    }
    get(db, id).await?;
    let touch: i64 = sqlx::query_scalar(
        "INSERT INTO interaction(contact_id, kind, summary, at) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(id)
    .bind(kind.as_str())
    .bind(s)
    .bind(at)
    .fetch_one(db.pool())
    .await
    .map_err(db_err)?;
    // A note is the person's own jotting, not contact with them.
    if kind != Kind::Note {
        sqlx::query(
            "UPDATE contact SET last_contacted = ? WHERE id = ? AND (last_contacted IS NULL OR last_contacted < ?)",
        )
        .bind(at)
        .bind(id)
        .bind(at)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    }
    Ok(touch)
}

/// A person's timeline, newest first.
pub async fn timeline(db: &Db, id: i64) -> Result<Vec<Touch>> {
    let rows = sqlx::query(
        "SELECT id, kind, summary, at FROM interaction WHERE contact_id = ? ORDER BY at DESC, id DESC",
    )
    .bind(id)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    Ok(rows
        .iter()
        .map(|r| Touch {
            id: r.get("id"),
            kind: Kind::parse(&r.get::<String, _>("kind")),
            summary: r.get("summary"),
            at: r.get("at"),
        })
        .collect())
}

/// Find a person already saved who looks like the same one: same email, else same profile URL,
/// else same name (ignoring case) at the same company.
pub async fn find_match(db: &Db, p: &NewPerson) -> Result<Option<i64>> {
    if let Some(e) = clean(&p.email) {
        let id: Option<i64> =
            sqlx::query_scalar("SELECT id FROM contact WHERE lower(email) = lower(?) LIMIT 1")
                .bind(e)
                .fetch_optional(db.pool())
                .await
                .map_err(db_err)?;
        if id.is_some() {
            return Ok(id);
        }
    }
    if let Some(u) = clean(&p.profile_url) {
        let id: Option<i64> =
            sqlx::query_scalar("SELECT id FROM contact WHERE profile_url = ? LIMIT 1")
                .bind(u)
                .fetch_optional(db.pool())
                .await
                .map_err(db_err)?;
        if id.is_some() {
            return Ok(id);
        }
    }
    let name = p.full_name.trim();
    let same_name: Vec<Person> = list(db, Some(name), None)
        .await?
        .into_iter()
        .filter(|x| x.full_name.eq_ignore_ascii_case(name))
        .collect();
    let want = clean(&p.company).map(|c| c.to_lowercase());
    Ok(same_name
        .into_iter()
        .find(|x| match (&want, &x.company) {
            (Some(w), Some(c)) => {
                c.to_lowercase().contains(w.as_str()) || w.contains(&c.to_lowercase())
            }
            _ => false,
        })
        .map(|x| x.id))
}

/// Merge `from` into `into`: fill blanks, combine tags and notes, move the timeline and any
/// outreach, then remove `from`.
pub async fn merge(db: &Db, into: i64, from: i64) -> Result<()> {
    if into == from {
        return Err(Error::Invalid("pick two different people".into()));
    }
    let a = get(db, into).await?;
    let b = get(db, from).await?;
    let mut tags = a.tags.clone();
    for t in &b.tags {
        if !tags.contains(t) {
            tags.push(t.clone());
        }
    }
    let notes = match (&a.notes, &b.notes) {
        (Some(x), Some(y)) if x != y => Some(format!("{x}\n\n{y}")),
        (Some(x), _) => Some(x.clone()),
        (None, y) => y.clone(),
    };
    let tags_json = serde_json::to_string(&tags).map_err(|e| Error::Other(e.to_string()))?;
    sqlx::query(
        "UPDATE contact SET email = COALESCE(email, ?), phone = COALESCE(phone, ?), \
         profile_url = COALESCE(profile_url, ?), how_met = COALESCE(how_met, ?), notes = ?, tags = ?, \
         last_contacted = (SELECT MAX(v) FROM (SELECT last_contacted AS v FROM contact WHERE id = ? \
           UNION ALL SELECT last_contacted FROM contact WHERE id = ?)) WHERE id = ?",
    )
    .bind(&b.email)
    .bind(&b.phone)
    .bind(&b.profile_url)
    .bind(&b.how_met)
    .bind(notes)
    .bind(tags_json)
    .bind(into)
    .bind(from)
    .bind(into)
    .execute(db.pool())
    .await
    .map_err(db_err)?;
    for sql in [
        "UPDATE interaction SET contact_id = ? WHERE contact_id = ?",
        "UPDATE outreach SET contact_id = ? WHERE contact_id = ?",
        "UPDATE contact_change SET contact_id = ? WHERE contact_id = ?",
    ] {
        sqlx::query(sql)
            .bind(into)
            .bind(from)
            .execute(db.pool())
            .await
            .map_err(db_err)?;
    }
    // Keep their current role only if the kept person has none.
    if a.company.is_none() {
        sqlx::query("UPDATE contact_role SET contact_id = ? WHERE contact_id = ?")
            .bind(into)
            .bind(from)
            .execute(db.pool())
            .await
            .map_err(db_err)?;
    }
    delete(db, from).await
}

/// A job change seen when a newer LinkedIn export was imported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub contact_id: i64,
    pub full_name: String,
    pub from_company: Option<String>,
    pub from_title: Option<String>,
    pub to_company: Option<String>,
    pub to_title: Option<String>,
    pub seen_at: String,
}

/// Recent job changes, newest first.
pub async fn recent_changes(db: &Db, limit: i64) -> Result<Vec<Change>> {
    let rows = sqlx::query(
        "SELECT ch.contact_id, c.full_name, ch.from_company, ch.from_title, ch.to_company, \
         ch.to_title, ch.seen_at FROM contact_change ch JOIN contact c ON c.id = ch.contact_id \
         ORDER BY ch.seen_at DESC, ch.id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    Ok(rows
        .iter()
        .map(|r| Change {
            contact_id: r.get("contact_id"),
            full_name: r.get("full_name"),
            from_company: r.get("from_company"),
            from_title: r.get("from_title"),
            to_company: r.get("to_company"),
            to_title: r.get("to_title"),
            seen_at: r.get("seen_at"),
        })
        .collect())
}

#[cfg(test)]
mod tests;
