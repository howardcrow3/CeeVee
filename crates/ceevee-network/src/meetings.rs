//! Meetings with people you know: plan one, get a short prep note, export a calendar hold, and
//! write what happened into their timeline afterwards.

use crate::people::{self, Kind};
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use sqlx::Row;

fn db_err(e: sqlx::Error) -> Error {
    Error::Other(e.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meeting {
    pub id: i64,
    pub contact_id: i64,
    pub person: String,
    pub title: String,
    /// ISO-8601 UTC.
    pub starts_at: String,
    pub minutes: i64,
    pub done: bool,
}

fn meeting(r: &sqlx::sqlite::SqliteRow) -> Meeting {
    Meeting {
        id: r.get("id"),
        contact_id: r.get("contact_id"),
        person: r.get("full_name"),
        title: r.get("title"),
        starts_at: r.get("starts_at"),
        minutes: r.get("minutes"),
        done: r.get::<i64, _>("done") != 0,
    }
}

const SELECT: &str =
    "SELECT m.id, m.contact_id, c.full_name, m.title, m.starts_at, m.minutes, m.done \
                      FROM meeting m JOIN contact c ON c.id = m.contact_id";

/// Plan a meeting. `starts_at` is ISO-8601 UTC, like `2026-12-03T17:00:00Z`.
pub async fn schedule(
    db: &Db,
    contact_id: i64,
    title: &str,
    starts_at: &str,
    minutes: i64,
) -> Result<i64> {
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::Invalid("give the meeting a title".into()));
    }
    if !valid_time(starts_at) {
        return Err(Error::Invalid(
            "the meeting time is not a date and time".into(),
        ));
    }
    people::get(db, contact_id).await?;
    sqlx::query_scalar(
        "INSERT INTO meeting(contact_id, title, starts_at, minutes) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(contact_id)
    .bind(title)
    .bind(starts_at)
    .bind(minutes.clamp(5, 480))
    .fetch_one(db.pool())
    .await
    .map_err(db_err)
}

/// `YYYY-MM-DDTHH:MM:SSZ`, which is what the app writes everywhere.
fn valid_time(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'Z'
        && b.iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7 | 10 | 13 | 16 | 19) || c.is_ascii_digit())
}

/// Meetings not yet done, soonest first (including ones that are already past, so none is lost).
pub async fn open(db: &Db) -> Result<Vec<Meeting>> {
    let rows = sqlx::query(&format!(
        "{SELECT} WHERE m.done = 0 ORDER BY m.starts_at, m.id"
    ))
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    Ok(rows.iter().map(meeting).collect())
}

pub async fn get(db: &Db, id: i64) -> Result<Meeting> {
    let row = sqlx::query(&format!("{SELECT} WHERE m.id = ?"))
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?;
    row.as_ref()
        .map(meeting)
        .ok_or_else(|| Error::NotFound(format!("meeting {id}")))
}

/// Mark a meeting held and put it in the person's timeline (this counts as being in touch).
/// `notes` can be empty.
pub async fn finish(db: &Db, id: i64, notes: &str) -> Result<()> {
    let m = get(db, id).await?;
    if m.done {
        return Ok(());
    }
    let summary = if notes.trim().is_empty() {
        m.title.clone()
    } else {
        format!("{}: {}", m.title, notes.trim())
    };
    people::add_touch(db, m.contact_id, Kind::Meeting, &summary, &m.starts_at).await?;
    sqlx::query("UPDATE meeting SET done = 1 WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn cancel(db: &Db, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM meeting WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

/// A short note to read before the meeting: who they are, how you know them, the last few
/// things in your timeline and any recent job change.
pub async fn prep(db: &Db, id: i64) -> Result<String> {
    let m = get(db, id).await?;
    let p = people::get(db, m.contact_id).await?;
    let mut out = String::new();
    out.push_str(&p.full_name);
    match (&p.title, &p.company) {
        (Some(t), Some(c)) => out.push_str(&format!(", {t} at {c}")),
        (None, Some(c)) => out.push_str(&format!(", {c}")),
        (Some(t), None) => out.push_str(&format!(", {t}")),
        _ => {}
    }
    out.push('\n');
    if let Some(h) = &p.how_met {
        out.push_str(&format!("How you met: {h}\n"));
    }
    if let Some(n) = &p.notes {
        out.push_str(&format!("Your notes: {n}\n"));
    }
    if let Some(c) = people::recent_changes(db, 50)
        .await?
        .into_iter()
        .find(|c| c.contact_id == p.id)
    {
        out.push_str(&format!(
            "Recent move: {} to {}\n",
            c.from_company.as_deref().unwrap_or("somewhere"),
            c.to_company.as_deref().unwrap_or("somewhere else")
        ));
    }
    let last = people::timeline(db, p.id).await?;
    if last.is_empty() {
        out.push_str("No earlier history in CeeVee.\n");
    } else {
        out.push_str("Last time:\n");
        for t in last.iter().take(3) {
            out.push_str(&format!(
                "- {} ({}): {}\n",
                &t.at[..t.at.len().min(10)],
                t.kind.as_str(),
                t.summary
            ));
        }
    }
    Ok(out)
}

fn ics_escape(s: &str) -> String {
    s.replace('\\', r"\\")
        .replace(';', r"\;")
        .replace(',', r"\,")
        .replace('\n', r"\n")
}

/// Add `minutes` to an ISO-8601 UTC time, as the compact form calendars use (`20261203T170000Z`).
fn compact(starts_at: &str, plus_minutes: i64) -> String {
    let n = |a: usize, b: usize| starts_at[a..b].parse::<i64>().unwrap_or(0);
    let (mut y, mut mo, mut d, mut h, mut mi) = (n(0, 4), n(5, 7), n(8, 10), n(11, 13), n(14, 16));
    mi += plus_minutes;
    h += mi / 60;
    mi %= 60;
    d += h / 24;
    h %= 24;
    loop {
        let len = match mo {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
            _ => 28,
        };
        if d <= len {
            break;
        }
        d -= len;
        mo += 1;
        if mo > 12 {
            mo = 1;
            y += 1;
        }
    }
    format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}{:02}Z", n(17, 19))
}

/// A calendar file (.ics) for the meeting, to open in any calendar app. Nothing is sent anywhere.
pub async fn ics(db: &Db, id: i64) -> Result<String> {
    let m = get(db, id).await?;
    let summary = ics_escape(&format!("{} with {}", m.title, m.person));
    Ok(format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//CeeVee//Network//EN\r\nBEGIN:VEVENT\r\n\
         UID:ceevee-meeting-{}@ceevee.local\r\nDTSTAMP:{}\r\nDTSTART:{}\r\nDTEND:{}\r\n\
         SUMMARY:{}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        m.id,
        compact(&m.starts_at, 0),
        compact(&m.starts_at, 0),
        compact(&m.starts_at, m.minutes),
        summary
    ))
}

#[cfg(test)]
mod tests;
