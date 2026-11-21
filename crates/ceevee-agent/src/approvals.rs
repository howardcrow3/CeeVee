//! The approvals queue and the run log, stored in SQLite (TECHNICAL_PLAN section 6.5).
//!
//! A call the policy queued becomes a `proposal` row of kind `tool_call`. The user accepts or
//! rejects it; accepting runs the tool once, as a user-initiated call. Finished runs are written to
//! `agent_run` and `agent_step` for the activity log.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde_json::{json, Value};
use sqlx::Row;

use crate::runner::{Outcome, RunResult, StepResult};
use crate::tools::Registry;

fn db_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    pub id: i64,
    pub run_id: Option<i64>,
    pub kind: String,
    pub payload: Value,
    pub state: String,
    pub created_at: String,
}

fn proposal_from(row: &sqlx::sqlite::SqliteRow) -> Result<Proposal> {
    let payload: String = row.get("payload");
    Ok(Proposal {
        id: row.get("id"),
        run_id: row.get("run_id"),
        kind: row.get("kind"),
        payload: serde_json::from_str(&payload)
            .map_err(|e| Error::Database(format!("proposal payload: {e}")))?,
        state: row.get("state"),
        created_at: row.get("created_at"),
    })
}

/// Write a finished run, its steps and the proposals for its queued calls in one transaction.
/// Returns the run id.
pub async fn save_run(db: &Db, agent: &str, trigger: &str, result: &RunResult) -> Result<i64> {
    let state = match result.outcome {
        Outcome::Final(_) => "done",
        Outcome::StepLimit => "step_limit",
        Outcome::Cancelled => "cancelled",
    };
    let mut tx = db.pool().begin().await.map_err(db_err)?;
    let run_id: i64 = sqlx::query_scalar(
        "INSERT INTO agent_run(agent, trigger, state, finished_at) \
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ','now')) RETURNING id",
    )
    .bind(agent)
    .bind(trigger)
    .bind(state)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;
    for s in &result.steps {
        let output = match &s.result {
            StepResult::Ran(v) => json!({"ran": v}),
            StepResult::Queued(r) => json!({"queued": r}),
            StepResult::Denied(r) => json!({"denied": r}),
            StepResult::Failed(e) => json!({"failed": e}),
        };
        sqlx::query(
            "INSERT INTO agent_step(run_id, tool, input, output, risk) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(run_id)
        .bind(&s.tool)
        .bind(s.args.to_string())
        .bind(output.to_string())
        .bind(format!("{:?}", s.risk).to_lowercase())
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }
    for q in &result.queued {
        let payload = json!({"agent": agent, "tool": q.tool, "args": q.args, "reason": q.reason});
        sqlx::query("INSERT INTO proposal(run_id, kind, payload) VALUES (?, 'tool_call', ?)")
            .bind(run_id)
            .bind(payload.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
    }
    tx.commit().await.map_err(db_err)?;
    Ok(run_id)
}

/// Add a proposal that is not a tool call, such as a profile change from the Profile agent.
pub async fn propose(db: &Db, run_id: Option<i64>, kind: &str, payload: &Value) -> Result<i64> {
    sqlx::query_scalar("INSERT INTO proposal(run_id, kind, payload) VALUES (?, ?, ?) RETURNING id")
        .bind(run_id)
        .bind(kind)
        .bind(payload.to_string())
        .fetch_one(db.pool())
        .await
        .map_err(db_err)
}

pub async fn pending(db: &Db) -> Result<Vec<Proposal>> {
    let rows = sqlx::query("SELECT * FROM proposal WHERE state = 'pending' ORDER BY id")
        .fetch_all(db.pool())
        .await
        .map_err(db_err)?;
    rows.iter().map(proposal_from).collect()
}

pub async fn pending_count(db: &Db) -> Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM proposal WHERE state = 'pending'")
        .fetch_one(db.pool())
        .await
        .map_err(db_err)
}

pub async fn get(db: &Db, id: i64) -> Result<Proposal> {
    let row = sqlx::query("SELECT * FROM proposal WHERE id = ?")
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .ok_or_else(|| Error::NotFound(format!("proposal {id}")))?;
    proposal_from(&row)
}

/// Move a pending proposal to a new state. Fails if someone already decided it, so a double click
/// cannot run a tool twice.
async fn decide(db: &Db, id: i64, state: &str) -> Result<Proposal> {
    let changed = sqlx::query(
        "UPDATE proposal SET state = ?, decided_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
         WHERE id = ? AND state = 'pending'",
    )
    .bind(state)
    .bind(id)
    .execute(db.pool())
    .await
    .map_err(db_err)?
    .rows_affected();
    if changed == 0 {
        get(db, id).await?; // NotFound if it does not exist
        return Err(Error::Invalid(format!("proposal {id} was already decided")));
    }
    get(db, id).await
}

pub async fn reject(db: &Db, id: i64) -> Result<Proposal> {
    decide(db, id, "rejected").await
}

/// Approve a queued tool call and run it once, as an action the user chose. Returns the tool's
/// result. Other kinds of proposal are applied by the code that owns them, after
/// [`approve_only`].
pub async fn approve_and_run(db: &Db, registry: &Registry, id: i64) -> Result<Value> {
    let p = get(db, id).await?;
    if p.kind != "tool_call" {
        return Err(Error::Invalid(format!(
            "proposal {id} is a {}, not a tool call",
            p.kind
        )));
    }
    let tool = p.payload["tool"]
        .as_str()
        .ok_or_else(|| Error::Invalid("proposal has no tool".into()))?;
    let spec = registry
        .get(tool)
        .ok_or_else(|| Error::NotFound(format!("tool {tool}")))?
        .spec();
    // The queue only holds calls the policy did not deny, but the never-list is checked again
    // here so a row edited by hand cannot run a forbidden tool.
    if crate::policy::is_never(spec.name) || spec.action.is_some_and(crate::policy::is_never) {
        return Err(Error::Invalid(format!(
            "{tool} is never available to an agent"
        )));
    }
    decide(db, id, "approved").await?;
    registry.call(tool, p.payload["args"].clone()).await
}

/// Mark a non-tool proposal approved; the caller then applies the change.
pub async fn approve_only(db: &Db, id: i64) -> Result<Proposal> {
    decide(db, id, "approved").await
}

/// Expire proposals still pending after `days`. Returns how many.
pub async fn expire_older_than(db: &Db, days: i64) -> Result<u64> {
    Ok(sqlx::query(
        "UPDATE proposal SET state = 'expired', decided_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
         WHERE state = 'pending' AND created_at < strftime('%Y-%m-%dT%H:%M:%fZ','now', ?)",
    )
    .bind(format!("-{days} days"))
    .execute(db.pool())
    .await
    .map_err(db_err)?
    .rows_affected())
}

#[cfg(test)]
mod tests;
