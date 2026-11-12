//! Job categories and rule-based scoring.
//!
//! A category says what the user is looking for (titles, keywords, places, things to avoid). Rule
//! scoring is cheap and runs on every new posting; the model's `score_match` task refines only the
//! postings that score well here. Each component present in the criteria (titles, keywords,
//! places) is worth an equal share of 100, and any avoid-word caps the score at 10.

use crate::{db_err, postings, words};
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Criteria {
    /// Job titles wanted ("product manager"); a posting title containing one of them matches.
    pub titles: Vec<String>,
    /// Words or phrases the posting should mention; the score is the share found.
    pub keywords: Vec<String>,
    /// Places wanted ("columbus"); matched against the posting's location.
    pub locations: Vec<String>,
    /// Counts "remote" in the title or location as a match for the places component.
    pub remote_ok: bool,
    /// Words or phrases that rule a posting out ("intern", "clearance").
    pub avoid: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub criteria: Criteria,
    pub priority: i64,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub value: f64,
    pub reasons: Vec<String>,
}

fn contains_phrase(haystack: &str, phrase: &str) -> bool {
    let p = words(phrase);
    !p.is_empty() && format!(" {haystack} ").contains(&format!(" {p} "))
}

/// Score one posting's text against criteria. Pure, so it is easy to test and to re-run.
pub fn score(c: &Criteria, title: &str, location: Option<&str>, body: Option<&str>) -> Score {
    let title_w = words(title);
    let loc_w = words(location.unwrap_or(""));
    let all_w = format!("{title_w} {loc_w} {}", words(body.unwrap_or("")));
    let mut reasons = Vec::new();

    if let Some(a) = c.avoid.iter().find(|a| contains_phrase(&all_w, a)) {
        reasons.push(format!("mentions \"{a}\", which you avoid"));
        return Score {
            value: 10.0,
            reasons,
        };
    }

    let mut parts = 0u32;
    let mut earned = 0.0;
    if !c.titles.is_empty() {
        parts += 1;
        match c.titles.iter().find(|t| contains_phrase(&title_w, t)) {
            Some(t) => {
                earned += 1.0;
                reasons.push(format!("title matches \"{t}\""));
            }
            None => reasons.push("title is not one you listed".into()),
        }
    }
    if !c.keywords.is_empty() {
        parts += 1;
        let found: Vec<&String> = c
            .keywords
            .iter()
            .filter(|k| contains_phrase(&all_w, k))
            .collect();
        earned += found.len() as f64 / c.keywords.len() as f64;
        if !found.is_empty() {
            reasons.push(format!(
                "mentions {}",
                found
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if found.len() < c.keywords.len() {
            let missing: Vec<&str> = c
                .keywords
                .iter()
                .filter(|k| !found.contains(k))
                .map(String::as_str)
                .collect();
            reasons.push(format!("does not mention {}", missing.join(", ")));
        }
    }
    if !c.locations.is_empty() || c.remote_ok {
        parts += 1;
        let place = c.locations.iter().find(|l| contains_phrase(&loc_w, l));
        let remote = c.remote_ok
            && (contains_phrase(&loc_w, "remote") || contains_phrase(&title_w, "remote"));
        if let Some(l) = place {
            earned += 1.0;
            reasons.push(format!("located in {l}"));
        } else if remote {
            earned += 1.0;
            reasons.push("remote".into());
        } else {
            reasons.push("not in a place you listed".into());
        }
    }
    let value = if parts == 0 {
        0.0
    } else {
        (earned / f64::from(parts) * 100.0).round()
    };
    Score { value, reasons }
}

fn from_row(r: &sqlx::sqlite::SqliteRow) -> Result<Category> {
    let criteria: String = r.get("criteria");
    Ok(Category {
        id: r.get("id"),
        name: r.get("name"),
        criteria: serde_json::from_str(&criteria).map_err(|e| Error::Other(e.to_string()))?,
        priority: r.get("priority"),
        active: r.get::<i64, _>("active") == 1,
    })
}

pub async fn create(db: &Db, name: &str, criteria: &Criteria) -> Result<Category> {
    if name.trim().is_empty() {
        return Err(Error::Invalid("a category needs a name".into()));
    }
    let id = sqlx::query("INSERT INTO job_category (name, criteria) VALUES (?, ?)")
        .bind(name.trim())
        .bind(serde_json::to_string(criteria).map_err(|e| Error::Other(e.to_string()))?)
        .execute(db.pool())
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => {
                Error::Invalid(format!("a category named {name} already exists"))
            }
            other => db_err(other),
        })?
        .last_insert_rowid();
    get(db, id).await
}

pub async fn get(db: &Db, id: i64) -> Result<Category> {
    let r =
        sqlx::query("SELECT id, name, criteria, priority, active FROM job_category WHERE id = ?")
            .bind(id)
            .fetch_optional(db.pool())
            .await
            .map_err(db_err)?
            .ok_or_else(|| Error::NotFound(format!("category {id}")))?;
    from_row(&r)
}

/// All categories, highest priority first.
pub async fn list(db: &Db) -> Result<Vec<Category>> {
    let rows = sqlx::query(
        "SELECT id, name, criteria, priority, active FROM job_category ORDER BY priority DESC, name",
    )
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    rows.iter().map(from_row).collect()
}

pub async fn set_active(db: &Db, id: i64, active: bool) -> Result<()> {
    get(db, id).await?;
    sqlx::query("UPDATE job_category SET active = ? WHERE id = ?")
        .bind(i64::from(active))
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Score a posting against every active category, store the rows in `posting_category`, and set
/// the posting's own score to the best one. Returns each category's score.
pub async fn score_posting(db: &Db, posting_id: i64) -> Result<Vec<(i64, Score)>> {
    let p = postings::get(db, posting_id).await?;
    let mut out = Vec::new();
    let mut best: Option<f64> = None;
    for c in list(db).await?.into_iter().filter(|c| c.active) {
        let s = score(
            &c.criteria,
            &p.title,
            p.location.as_deref(),
            p.body.as_deref(),
        );
        sqlx::query(
            "INSERT INTO posting_category (posting_id, category_id, score, score_reasons, scored_with) \
             VALUES (?, ?, ?, ?, 'rules@1') \
             ON CONFLICT(posting_id, category_id) DO UPDATE SET \
             score = excluded.score, score_reasons = excluded.score_reasons, scored_with = excluded.scored_with",
        )
        .bind(posting_id)
        .bind(c.id)
        .bind(s.value)
        .bind(serde_json::to_string(&s.reasons).map_err(|e| Error::Other(e.to_string()))?)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
        best = Some(best.map_or(s.value, |b| b.max(s.value)));
        out.push((c.id, s));
    }
    sqlx::query("UPDATE posting SET score = ? WHERE id = ?")
        .bind(best)
        .bind(posting_id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(out)
}

#[cfg(test)]
mod tests;
