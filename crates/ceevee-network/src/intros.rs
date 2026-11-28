//! Warm introductions: who you know at a company (now or before), the best person to ask first,
//! a short message to start from, and a record of who you asked and what came of it.
//!
//! Nothing is sent from here. The message is text for the person to read, change and send
//! themselves, which keeps their accounts safe and the wording theirs.

use crate::people::{self, Kind, Tier};
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use sqlx::Row;

fn db_err(e: sqlx::Error) -> Error {
    Error::Other(e.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    /// Works there now.
    Works,
    /// Used to work there, per a job move seen in a newer LinkedIn export.
    Worked,
}

/// Someone who could help with a company.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub contact_id: i64,
    pub name: String,
    pub title: Option<String>,
    pub tier: Tier,
    pub link: Link,
    pub last_contacted: Option<String>,
    /// `asked`, `replied` or `referred` when you have already asked them about this company.
    pub asked: Option<String>,
}

fn tier_rank(t: Tier) -> u8 {
    match t {
        Tier::Close => 0,
        Tier::Warm => 1,
        Tier::Weak => 2,
    }
}

/// People who could introduce you at `company_id`, best first: closer people before weaker ones,
/// current employees before former ones, and people you have not already asked after those you
/// have not.
pub async fn paths(db: &Db, company_id: i64) -> Result<Vec<Path>> {
    let name: String = sqlx::query_scalar("SELECT name FROM company WHERE id = ?")
        .bind(company_id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .ok_or_else(|| Error::NotFound(format!("company {company_id}")))?;
    let rows = sqlx::query(
        "SELECT c.id, c.full_name, r.title, c.tier, c.last_contacted, 1 AS works \
           FROM contact c JOIN contact_role r ON r.contact_id = c.id \
          WHERE r.company_id = ?1 AND r.is_current = 1 \
         UNION \
         SELECT c.id, c.full_name, ch.from_title, c.tier, c.last_contacted, 0 AS works \
           FROM contact c JOIN contact_change ch ON ch.contact_id = c.id \
          WHERE lower(ch.from_company) = lower(?2) \
            AND c.id NOT IN (SELECT contact_id FROM contact_role WHERE company_id = ?1 AND is_current = 1)",
    )
    .bind(company_id)
    .bind(&name)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    let mut out: Vec<Path> = Vec::new();
    for r in &rows {
        let id: i64 = r.get("id");
        if out.iter().any(|p| p.contact_id == id) {
            continue;
        }
        let tier = match r.get::<String, _>("tier").as_str() {
            "close" => Tier::Close,
            "weak" => Tier::Weak,
            _ => Tier::Warm,
        };
        let asked: Option<String> = sqlx::query_scalar(
            "SELECT status FROM outreach WHERE contact_id = ? AND message LIKE ? \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(id)
        .bind(format!("[{company_id}]%"))
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?;
        out.push(Path {
            contact_id: id,
            name: r.get("full_name"),
            title: r.get("title"),
            tier,
            link: if r.get::<i64, _>("works") == 1 {
                Link::Works
            } else {
                Link::Worked
            },
            last_contacted: r.get("last_contacted"),
            asked,
        });
    }
    out.sort_by(|a, b| {
        a.asked
            .is_some()
            .cmp(&b.asked.is_some())
            .then(tier_rank(a.tier).cmp(&tier_rank(b.tier)))
            .then((a.link == Link::Worked).cmp(&(b.link == Link::Worked)))
            .then(a.name.cmp(&b.name))
    });
    Ok(out)
}

/// What to ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// Put in a word, or refer you for a specific job.
    Referral,
    /// A short chat about the company and the team.
    Chat,
}

/// A short message to start from. `job_title` can be empty for a general chat.
pub async fn draft(
    db: &Db,
    contact_id: i64,
    company: &str,
    job_title: &str,
    ask: Ask,
    now: &str,
) -> Result<String> {
    let p = people::get(db, contact_id).await?;
    let first = p.full_name.split_whitespace().next().unwrap_or("there");
    let long_silent = match &p.last_contacted {
        None => true,
        Some(t) => {
            let days: Option<i64> =
                sqlx::query_scalar("SELECT CAST(julianday(?1) - julianday(?2) AS INTEGER)")
                    .bind(now)
                    .bind(t)
                    .fetch_one(db.pool())
                    .await
                    .map_err(db_err)?;
            days.unwrap_or(999) > 180
        }
    };
    let mut s = format!("Hi {first},\n\n");
    if long_silent {
        s.push_str("It has been a while, and I hope things are going well. ");
    } else {
        s.push_str("Hope you are doing well. ");
    }
    let works_there = p
        .company
        .as_deref()
        .is_some_and(|c| c.eq_ignore_ascii_case(company));
    let role = if job_title.trim().is_empty() {
        String::new()
    } else {
        format!(" for the {} role", job_title.trim())
    };
    match (ask, works_there) {
        (Ask::Referral, true) => s.push_str(&format!(
            "I am applying to {company}{role} and noticed you are there. Would you be open to \
             putting in a word or referring me? I can send my resume and a few lines on why I \
             think it fits, so it takes you no time.\n"
        )),
        (Ask::Referral, false) => s.push_str(&format!(
            "I am applying to {company}{role}. I remember you used to work there. Do you know \
             anyone on the team I could talk to, and would you be open to introducing me?\n"
        )),
        (Ask::Chat, true) => s.push_str(&format!(
            "I am looking at {company}{role} and would love to hear what it is like to work \
             there. Could we do a 15 minute call in the next couple of weeks?\n"
        )),
        (Ask::Chat, false) => s.push_str(&format!(
            "I am looking at {company}{role}, and since you used to work there I would value \
             your view on the team and culture. Could we do a 15 minute call in the next couple \
             of weeks?\n"
        )),
    }
    s.push_str("\nNo pressure at all if the timing is not right.\n\nThanks so much,\n");
    Ok(s)
}

/// Record that you asked someone about a company (you sent the message yourself). This puts the
/// ask in their timeline and starts the clock for keep-in-touch reminders.
pub async fn record_ask(
    db: &Db,
    contact_id: i64,
    company_id: i64,
    message: &str,
    now: &str,
) -> Result<i64> {
    people::get(db, contact_id).await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO outreach(contact_id, message, status, asked_at) VALUES (?, ?, 'asked', ?) RETURNING id",
    )
    .bind(contact_id)
    .bind(format!("[{company_id}]{message}"))
    .bind(now)
    .fetch_one(db.pool())
    .await
    .map_err(db_err)?;
    people::add_touch(
        db,
        contact_id,
        Kind::Message,
        "Asked about an introduction",
        now,
    )
    .await?;
    Ok(id)
}

/// Update how an ask turned out: `replied` or `referred`.
pub async fn set_outcome(db: &Db, outreach_id: i64, status: &str, now: &str) -> Result<()> {
    if !matches!(status, "replied" | "referred") {
        return Err(Error::Invalid("outcome must be replied or referred".into()));
    }
    let n = sqlx::query("UPDATE outreach SET status = ?, replied_at = ? WHERE id = ?")
        .bind(status)
        .bind(now)
        .bind(outreach_id)
        .execute(db.pool())
        .await
        .map_err(db_err)?
        .rows_affected();
    if n == 0 {
        return Err(Error::NotFound(format!("ask {outreach_id}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
