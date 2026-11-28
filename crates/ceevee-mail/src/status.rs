//! Turn a label into a status change, honoring the user's policy for `application.status`.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use ceevee_search::tracking::{self, Cause, Status};
use serde_json::{json, Value};

use crate::label::Kind;

fn db_err(e: impl std::fmt::Display) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Off,
    Ask,
    Auto,
}

fn rank(s: Status) -> u8 {
    match s {
        Status::Draft => 0,
        Status::Sent => 1,
        Status::Screening => 2,
        Status::Interview => 3,
        Status::Offer => 4,
        _ => 5,
    }
}

/// The status an email suggests for an application now in `current`. Mail only moves an
/// application forward, or closes it with a rejection; an acknowledgement is the one case that
/// can leave a draft (the user clearly applied).
pub fn suggest(kind: Kind, current: Status) -> Option<Status> {
    suggest_from_status(current, kind.status()?)
}

/// What the user chose for `application.status`: a per-category row beats the global row. With no
/// row, forward moves apply on their own and an offer or a rejection waits for the user to
/// confirm, because those two are the ones people want to read first.
pub async fn mode_for(db: &Db, category: Option<i64>, to: Status) -> Result<Mode> {
    let rows: Vec<(Option<i64>, String)> =
        sqlx::query_as("SELECT category_id, action_modes FROM policy")
            .fetch_all(db.pool())
            .await
            .map_err(db_err)?;
    let pick = |scope: Option<i64>| -> Option<String> {
        rows.iter()
            .find(|(c, _)| *c == scope)
            .and_then(|(_, m)| serde_json::from_str::<Value>(m).ok())
            .and_then(|v| v.get("application.status")?.as_str().map(str::to_string))
    };
    let chosen = category.and_then(|c| pick(Some(c))).or_else(|| pick(None));
    Ok(match chosen.as_deref() {
        Some("off") => Mode::Off,
        Some("auto") => Mode::Auto,
        Some("ask") => Mode::Ask,
        _ if matches!(to, Status::Offer | Status::Rejected) => Mode::Ask,
        _ => Mode::Auto,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Applied(Status),
    /// Waiting in the approvals queue as this proposal.
    Proposed(i64),
    Nothing,
}

/// Change the status now, or queue a proposal, or do nothing, according to the policy.
pub async fn apply(
    db: &Db,
    application_id: i64,
    message_db_id: i64,
    to: Status,
    reason: &str,
    now: &str,
) -> Result<Outcome> {
    let app = tracking::get(db, application_id).await?;
    let Some(to) = suggest_from_status(app.status, to) else {
        return Ok(Outcome::Nothing);
    };
    match mode_for(db, app.category_id, to).await? {
        Mode::Off => Ok(Outcome::Nothing),
        Mode::Auto => {
            tracking::set_status(db, application_id, to, Cause::Email, now).await?;
            Ok(Outcome::Applied(to))
        }
        Mode::Ask => {
            let open: Option<i64> = sqlx::query_scalar(
                "SELECT id FROM proposal WHERE kind = 'mail_status' AND state = 'pending' \
                 AND json_extract(payload, '$.application_id') = ? \
                 AND json_extract(payload, '$.to') = ?",
            )
            .bind(application_id)
            .bind(to.as_str())
            .fetch_optional(db.pool())
            .await
            .map_err(db_err)?;
            if let Some(id) = open {
                return Ok(Outcome::Proposed(id));
            }
            let payload = json!({
                "application_id": application_id, "to": to.as_str(),
                "message_id": message_db_id, "reason": reason,
            });
            let id: i64 = sqlx::query_scalar(
                "INSERT INTO proposal(kind, payload) VALUES ('mail_status', ?) RETURNING id",
            )
            .bind(payload.to_string())
            .fetch_one(db.pool())
            .await
            .map_err(db_err)?;
            Ok(Outcome::Proposed(id))
        }
    }
}

fn suggest_from_status(current: Status, to: Status) -> Option<Status> {
    if current.is_closed() || !current.can_move_to(to) {
        return None;
    }
    (to == Status::Rejected || rank(to) > rank(current)).then_some(to)
}

/// Carry out an approved `mail_status` proposal.
pub async fn apply_proposal(db: &Db, payload: &Value, now: &str) -> Result<()> {
    let id = payload["application_id"]
        .as_i64()
        .ok_or_else(|| Error::Invalid("proposal has no application".into()))?;
    let to = Status::parse(payload["to"].as_str().unwrap_or_default())?;
    tracking::set_status(db, id, to, Cause::Email, now).await?;
    Ok(())
}
