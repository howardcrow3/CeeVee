//! Application tracking: the board of applications, their status history and follow-up reminders.
//!
//! Every status change writes a `status_event` row saying who caused it (the user, an email the
//! app read, auto-fill, or the agent), so the history can always be explained.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde::{Deserialize, Serialize};
use sqlx::Row;

fn db_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Draft,
    Sent,
    Screening,
    Interview,
    Offer,
    Accepted,
    Rejected,
    Declined,
    Withdrawn,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Sent => "sent",
            Self::Screening => "screening",
            Self::Interview => "interview",
            Self::Offer => "offer",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Declined => "declined",
            Self::Withdrawn => "withdrawn",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "draft" => Self::Draft,
            "sent" => Self::Sent,
            "screening" => Self::Screening,
            "interview" => Self::Interview,
            "offer" => Self::Offer,
            "accepted" => Self::Accepted,
            "rejected" => Self::Rejected,
            "declined" => Self::Declined,
            "withdrawn" => Self::Withdrawn,
            other => return Err(Error::Invalid(format!("unknown status {other}"))),
        })
    }

    /// Finished applications no longer need follow-up.
    pub fn is_closed(self) -> bool {
        matches!(
            self,
            Self::Accepted | Self::Rejected | Self::Declined | Self::Withdrawn
        )
    }

    /// Whether moving to `to` makes sense. Moving forward or sideways through the pipeline is
    /// allowed (people skip stages: a recruiter may call before any screening), closing is allowed
    /// from any open stage, and a closed application can only be reopened by an explicit
    /// manual change to `draft` or `sent`... which the UI offers as "undo".
    pub fn can_move_to(self, to: Self) -> bool {
        if self == to {
            return false;
        }
        if self.is_closed() {
            return matches!(to, Self::Draft | Self::Sent);
        }
        if to == Self::Draft {
            return false;
        }
        if to == Self::Accepted {
            return self == Self::Offer;
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    Manual,
    Email,
    Autofill,
    Agent,
}

impl Cause {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Email => "email",
            Self::Autofill => "autofill",
            Self::Agent => "agent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Application {
    pub id: i64,
    pub posting_id: Option<i64>,
    pub company_id: Option<i64>,
    pub category_id: Option<i64>,
    pub status: Status,
    pub applied_at: Option<String>,
    pub next_interview_at: Option<String>,
    pub board_order: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct NewApplication {
    pub posting_id: Option<i64>,
    pub company_id: Option<i64>,
    pub category_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusEvent {
    pub from: Option<Status>,
    pub to: Status,
    pub cause: String,
    pub at: String,
}

const COLUMNS: &str = "id, posting_id, company_id, category_id, status, applied_at, \
                       next_interview_at, board_order, created_at";

fn from_row(r: &sqlx::sqlite::SqliteRow) -> Result<Application> {
    Ok(Application {
        id: r.get("id"),
        posting_id: r.get("posting_id"),
        company_id: r.get("company_id"),
        category_id: r.get("category_id"),
        status: Status::parse(r.get::<String, _>("status").as_str())?,
        applied_at: r.get("applied_at"),
        next_interview_at: r.get("next_interview_at"),
        board_order: r.get("board_order"),
        created_at: r.get("created_at"),
    })
}

/// Create a draft application at the bottom of the draft column. `now` is an ISO-8601 UTC timestamp.
pub async fn create(db: &Db, new: &NewApplication, now: &str) -> Result<Application> {
    let order = next_order(db, Status::Draft).await?;
    let id = sqlx::query(
        "INSERT INTO application (posting_id, company_id, category_id, board_order) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(new.posting_id)
    .bind(new.company_id)
    .bind(new.category_id)
    .bind(order)
    .execute(db.pool())
    .await
    .map_err(db_err)?
    .last_insert_rowid();
    sqlx::query("INSERT INTO status_event (application_id, from_status, to_status, cause, at) VALUES (?, NULL, 'draft', 'manual', ?)")
        .bind(id)
        .bind(now)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    get(db, id).await
}

async fn next_order(db: &Db, status: Status) -> Result<i64> {
    let row = sqlx::query(
        "SELECT COALESCE(MAX(board_order), -1) + 1 AS n FROM application WHERE status = ?",
    )
    .bind(status.as_str())
    .fetch_one(db.pool())
    .await
    .map_err(db_err)?;
    Ok(row.get("n"))
}

pub async fn get(db: &Db, id: i64) -> Result<Application> {
    let row = sqlx::query(&format!("SELECT {COLUMNS} FROM application WHERE id = ?"))
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .ok_or_else(|| Error::NotFound(format!("application {id}")))?;
    from_row(&row)
}

#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub status: Option<Status>,
    pub category_id: Option<i64>,
    pub company_id: Option<i64>,
}

/// Applications in board order (by column, then position), optionally filtered.
pub async fn list(db: &Db, filter: &Filter) -> Result<Vec<Application>> {
    let rows = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM application \
         WHERE (?1 IS NULL OR status = ?1) AND (?2 IS NULL OR category_id = ?2) \
           AND (?3 IS NULL OR company_id = ?3) \
         ORDER BY board_order, id"
    ))
    .bind(filter.status.map(Status::as_str))
    .bind(filter.category_id)
    .bind(filter.company_id)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    rows.iter().map(from_row).collect()
}

/// Move an application to a new status and record why. `now` is an ISO-8601 UTC timestamp.
/// Moving to `sent` stamps `applied_at` the first time.
pub async fn set_status(
    db: &Db,
    id: i64,
    to: Status,
    cause: Cause,
    now: &str,
) -> Result<Application> {
    let current = get(db, id).await?;
    if !current.status.can_move_to(to) {
        return Err(Error::Invalid(format!(
            "cannot move an application from {} to {}",
            current.status.as_str(),
            to.as_str()
        )));
    }
    let order = next_order(db, to).await?;
    let mut tx = db.pool().begin().await.map_err(db_err)?;
    sqlx::query(
        "UPDATE application SET status = ?, board_order = ?, \
         applied_at = CASE WHEN ? = 'sent' AND applied_at IS NULL THEN ? ELSE applied_at END \
         WHERE id = ?",
    )
    .bind(to.as_str())
    .bind(order)
    .bind(to.as_str())
    .bind(now)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    sqlx::query(
        "INSERT INTO status_event (application_id, from_status, to_status, cause, at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(current.status.as_str())
    .bind(to.as_str())
    .bind(cause.as_str())
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    get(db, id).await
}

/// The application's status history, oldest first.
pub async fn history(db: &Db, id: i64) -> Result<Vec<StatusEvent>> {
    let rows = sqlx::query(
        "SELECT from_status, to_status, cause, at FROM status_event \
         WHERE application_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    rows.iter()
        .map(|r| {
            let from: Option<String> = r.get("from_status");
            Ok(StatusEvent {
                from: from.as_deref().map(Status::parse).transpose()?,
                to: Status::parse(r.get::<String, _>("to_status").as_str())?,
                cause: r.get("cause"),
                at: r.get("at"),
            })
        })
        .collect()
}

pub async fn set_next_interview(db: &Db, id: i64, at: Option<&str>) -> Result<()> {
    get(db, id).await?;
    sqlx::query("UPDATE application SET next_interview_at = ? WHERE id = ?")
        .bind(at)
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Reorder one column. `ids` is the new order of that column's applications; ids that are not in
/// the column are rejected so a stale drag cannot scramble another column.
pub async fn reorder(db: &Db, status: Status, ids: &[i64]) -> Result<()> {
    let mut tx = db.pool().begin().await.map_err(db_err)?;
    for (pos, id) in ids.iter().enumerate() {
        let n = sqlx::query("UPDATE application SET board_order = ? WHERE id = ? AND status = ?")
            .bind(pos as i64)
            .bind(id)
            .bind(status.as_str())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(Error::Invalid(format!(
                "application {id} is not in {}",
                status.as_str()
            )));
        }
    }
    tx.commit().await.map_err(db_err)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reminder {
    pub application_id: i64,
    pub status: Status,
    pub quiet_days: i64,
}

/// Applications that have gone quiet: open, past the draft stage, and with no status change for
/// at least `after_days`. Offers are included since they usually have a deadline. `now` is an
/// ISO-8601 UTC timestamp.
pub async fn reminders(db: &Db, now: &str, after_days: i64) -> Result<Vec<Reminder>> {
    let rows = sqlx::query(
        "SELECT a.id AS id, a.status AS status, \
                CAST(julianday(?1) - julianday(MAX(e.at)) AS INTEGER) AS quiet \
         FROM application a JOIN status_event e ON e.application_id = a.id \
         WHERE a.status IN ('sent','screening','interview','offer') \
         GROUP BY a.id HAVING quiet >= ?2 ORDER BY quiet DESC, a.id",
    )
    .bind(now)
    .bind(after_days)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    rows.iter()
        .map(|r| {
            Ok(Reminder {
                application_id: r.get("id"),
                status: Status::parse(r.get::<String, _>("status").as_str())?,
                quiet_days: r.get("quiet"),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
