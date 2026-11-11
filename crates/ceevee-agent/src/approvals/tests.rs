use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::runner::{QueuedCall, StepRecord};
use crate::tools::{FnTool, Risk, ToolFuture, ToolSpec};

fn registry(runs: Arc<AtomicUsize>) -> Registry {
    let mut r = Registry::new();
    r.register(FnTool::new(
        ToolSpec {
            name: "apply.submit",
            description: "",
            risk: Risk::Act,
            input_schema: json!({"type":"object","required":["id"],"properties":{"id":{"type":"integer"}}}),
            action: Some("apply.submit"),
            untrusted_output: false,
        },
        move |v| -> ToolFuture {
            runs.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move { Ok(json!({"submitted": v["id"]})) })
        },
    ))
    .unwrap();
    r
}

fn result_with_queued() -> RunResult {
    let args = json!({"id": 9});
    RunResult {
        outcome: Outcome::Final("waiting".into()),
        steps: vec![StepRecord {
            index: 1,
            tool: "apply.submit".into(),
            args: args.clone(),
            risk: Risk::Act,
            result: StepResult::Queued("apply.submit needs your approval".into()),
        }],
        queued: vec![QueuedCall {
            tool: "apply.submit".into(),
            args,
            reason: "apply.submit needs your approval".into(),
        }],
    }
}

#[tokio::test]
async fn saves_the_run_its_steps_and_a_proposal() {
    let db = Db::open_in_memory().await.unwrap();
    let run_id = save_run(&db, "apply", "user", &result_with_queued())
        .await
        .unwrap();
    let steps: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_step WHERE run_id = ?")
        .bind(run_id)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(steps, 1);
    let list = pending(&db).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].payload["tool"], "apply.submit");
    assert_eq!(list[0].run_id, Some(run_id));
    assert_eq!(pending_count(&db).await.unwrap(), 1);
}

#[tokio::test]
async fn approving_runs_the_tool_exactly_once() {
    let db = Db::open_in_memory().await.unwrap();
    save_run(&db, "apply", "user", &result_with_queued())
        .await
        .unwrap();
    let id = pending(&db).await.unwrap()[0].id;
    let runs = Arc::new(AtomicUsize::new(0));
    let reg = registry(runs.clone());
    let out = approve_and_run(&db, &reg, id).await.unwrap();
    assert_eq!(out["submitted"], 9);
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    assert_eq!(pending_count(&db).await.unwrap(), 0);
    let again = approve_and_run(&db, &reg, id).await.unwrap_err();
    assert!(again.to_string().contains("already decided"), "{again}");
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn rejecting_does_not_run_anything() {
    let db = Db::open_in_memory().await.unwrap();
    save_run(&db, "apply", "user", &result_with_queued())
        .await
        .unwrap();
    let id = pending(&db).await.unwrap()[0].id;
    let p = reject(&db, id).await.unwrap();
    assert_eq!(p.state, "rejected");
    assert!(reject(&db, id).await.is_err());
    assert!(matches!(get(&db, 999).await, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn other_kinds_are_approved_without_running() {
    let db = Db::open_in_memory().await.unwrap();
    let id = propose(
        &db,
        None,
        "profile_change",
        &json!({"field": "title", "value": "Analyst"}),
    )
    .await
    .unwrap();
    let reg = registry(Arc::default());
    assert!(approve_and_run(&db, &reg, id).await.is_err()); // not a tool call
    assert_eq!(pending_count(&db).await.unwrap(), 1);
    let p = approve_only(&db, id).await.unwrap();
    assert_eq!(p.state, "approved");
}

#[tokio::test]
async fn old_pending_proposals_expire() {
    let db = Db::open_in_memory().await.unwrap();
    let id = propose(&db, None, "profile_change", &json!({}))
        .await
        .unwrap();
    sqlx::query("UPDATE proposal SET created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now','-10 days') WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .unwrap();
    propose(&db, None, "profile_change", &json!({}))
        .await
        .unwrap();
    assert_eq!(expire_older_than(&db, 7).await.unwrap(), 1);
    assert_eq!(get(&db, id).await.unwrap().state, "expired");
    assert_eq!(pending_count(&db).await.unwrap(), 1);
}
