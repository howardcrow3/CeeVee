//! Start tracking a job that an email revealed: a recruiter's note, an application receipt for a
//! job added nowhere else, an interview invitation or a rejection. The job is created as
//! "pending review" so the user can confirm or dismiss it; nothing is deleted behind their back.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use ceevee_search::postings::{self, Ingest, NewPosting};
use ceevee_search::tracking::{self, Cause, NewApplication, Status};
use serde::Serialize;
use sqlx::Row;

use crate::label::{clip, Label};
use crate::parse::ParsedMail;

fn db_err(e: impl std::fmt::Display) -> Error {
    Error::Database(e.to_string())
}

/// What a message did to the job list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Created {
    pub application_id: i64,
    /// `true` when this message started the job; `false` when it belongs to one already known.
    pub new: bool,
}

/// Lowercase letters and digits only, for comparing names.
fn key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Note the sender's domain on the company when the domain plainly belongs to it ("brightpath.com"
/// for Brightpath Health), so the company's next email links without any guessing. A staffing
/// agency's domain does not match the client's name and is left alone.
async fn remember_domain(db: &Db, company_id: i64, company: &str, mail: &ParsedMail) -> Result<()> {
    let Some(domain) = mail.from_domain() else {
        return Ok(());
    };
    let Some(guess) = crate::extract::company_from_domain(&domain) else {
        return Ok(());
    };
    let (g, c) = (key(&guess), key(company));
    if g.len() < 4 || !(c.starts_with(&g) || g.starts_with(&c)) {
        return Ok(());
    }
    let current: String = sqlx::query_scalar("SELECT email_domains FROM company WHERE id = ?")
        .bind(company_id)
        .fetch_one(db.pool())
        .await
        .map_err(db_err)?;
    let mut list: Vec<String> = serde_json::from_str(&current).unwrap_or_default();
    if !list.contains(&domain) {
        list.push(domain);
        sqlx::query("UPDATE company SET email_domains = ? WHERE id = ?")
            .bind(serde_json::to_string(&list).unwrap_or_else(|_| "[]".into()))
            .bind(company_id)
            .execute(db.pool())
            .await
            .map_err(db_err)?;
    }
    Ok(())
}

/// Title shown when an email names the company but not the job.
pub const UNKNOWN_TITLE: &str = "Role not stated in email";

/// Create (or find) the job this email is about. Returns `None` when the email is not enough to
/// start a job (no company, a kind that carries no status, a blocked company) or when the user
/// already dismissed this job. The caller links the message to the result.
pub async fn from_mail(
    db: &Db,
    mail: &ParsedMail,
    lab: &Label,
    now: &str,
) -> Result<Option<Created>> {
    let Some(start) = lab.kind.starts_job_as() else {
        return Ok(None);
    };
    let Some(company) = lab
        .company
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
    else {
        return Ok(None);
    };
    let title = lab
        .job_title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or(UNKNOWN_TITLE);
    let posting = NewPosting {
        source: "email".into(),
        // The same company and title always map to the same posting, so a second email about the
        // job finds the first one's record instead of starting another.
        url: format!("email:{}:{}", key(company), key(title)),
        company: company.into(),
        title: title.into(),
        location: lab.location.clone(),
        level: None,
        pay: None,
        body: Some(clip(&mail.body, 4000).to_string()),
    };
    let posting_id = match postings::ingest(db, &posting, now).await? {
        Ingest::New(id) | Ingest::Duplicate(id) => id,
        Ingest::Blocked => return Ok(None),
    };
    let existing: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, review FROM application WHERE posting_id = ? ORDER BY id LIMIT 1",
    )
    .bind(posting_id)
    .fetch_optional(db.pool())
    .await
    .map_err(db_err)?;
    if let Some((id, review)) = existing {
        return Ok((review != "dismissed").then_some(Created {
            application_id: id,
            new: false,
        }));
    }
    sqlx::query("UPDATE posting SET state = 'saved' WHERE id = ? AND state = 'new'")
        .bind(posting_id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    let company_id = postings::get(db, posting_id).await?.company_id;
    let app = tracking::create(
        db,
        &NewApplication {
            posting_id: Some(posting_id),
            company_id,
            category_id: None,
        },
        now,
    )
    .await?;
    sqlx::query("UPDATE application SET origin = 'email', review = 'pending' WHERE id = ?")
        .bind(app.id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    if start != Status::Draft {
        tracking::set_status(db, app.id, start, Cause::Email, now).await?;
    }
    if let Some(c) = company_id {
        remember_domain(db, c, company, mail).await?;
    }
    Ok(Some(Created {
        application_id: app.id,
        new: true,
    }))
}

/// A job CeeVee found in email that is waiting for the user to confirm or dismiss it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pending {
    pub application_id: i64,
    pub company: String,
    pub title: String,
    pub location: Option<String>,
    pub status: String,
    /// The message that started the job.
    pub message_id: Option<i64>,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub received_at: Option<String>,
    pub summary: Option<String>,
}

/// Jobs found in email and not yet confirmed or dismissed, newest first.
pub async fn pending(db: &Db) -> Result<Vec<Pending>> {
    let rows = sqlx::query(
        "SELECT a.id, a.status, COALESCE(c.name, '') AS company, COALESCE(p.title, '') AS title, \
                p.location, m.id AS message_id, m.subject, \
                COALESCE(m.from_name, m.from_addr) AS sender, m.received_at, m.summary \
         FROM application a \
         LEFT JOIN company c ON c.id = a.company_id \
         LEFT JOIN posting p ON p.id = a.posting_id \
         LEFT JOIN email_message m \
           ON m.id = (SELECT MIN(id) FROM email_message WHERE application_id = a.id) \
         WHERE a.review = 'pending' ORDER BY a.id DESC",
    )
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    Ok(rows
        .iter()
        .map(|r| Pending {
            application_id: r.get("id"),
            company: r.get("company"),
            title: r.get("title"),
            location: r.get("location"),
            status: r.get("status"),
            message_id: r.get("message_id"),
            subject: r.get("subject"),
            from: r.get("sender"),
            received_at: r.get("received_at"),
            summary: r.get("summary"),
        })
        .collect())
}

async fn set_review(db: &Db, id: i64, review: &str) -> Result<()> {
    let n = sqlx::query("UPDATE application SET review = ? WHERE id = ? AND origin = 'email'")
        .bind(review)
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?
        .rows_affected();
    if n == 0 {
        return Err(Error::NotFound(format!("job found in email {id}")));
    }
    Ok(())
}

/// Keep a job found in email: it becomes an ordinary application.
pub async fn confirm(db: &Db, id: i64) -> Result<()> {
    set_review(db, id, "confirmed").await
}

/// Throw away a job found in email. It leaves the board, its messages become unlinked mail in the
/// Inbox, and a later email about the same job does not bring it back.
pub async fn dismiss(db: &Db, id: i64) -> Result<()> {
    set_review(db, id, "dismissed").await?;
    sqlx::query("UPDATE email_message SET application_id = NULL WHERE application_id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}
