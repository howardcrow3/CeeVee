//! End-to-end tests across modules: database, task framework, agent loop, policy, approvals and
//! scheduler, with a scripted model in place of llama.cpp. These run on every PR and again on main
//! after each merge (`.github/workflows/post-merge.yml`).

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ceevee_agent::approvals;
use ceevee_agent::policy::{Mode, Policy};
use ceevee_agent::runner::{AgentProfile, Harness, NoLog, Outcome, RunContext, RunResult};
use ceevee_agent::scheduler;
use ceevee_agent::tools::{FnTool, Registry, Risk, ToolFuture, ToolSpec};
use ceevee_ai::task::{self, EXTRACT_POSTING};
use ceevee_ai::{GenerateRequest, LlmBackend, ModelWorker, Priority};
use ceevee_core::schedule::{Schedule, Task};
use ceevee_core::{Error, Event, EventBus, Result};
use ceevee_db::Db;
use chrono::{Local, TimeZone};
use serde_json::{json, Value};

/// A fake engine that answers from a script and records every prompt it was given.
struct Scripted {
    replies: Vec<String>,
    prompts: Arc<Mutex<Vec<String>>>,
    loaded: bool,
}

impl LlmBackend for Scripted {
    fn load(&mut self) -> Result<()> {
        self.loaded = true;
        Ok(())
    }
    fn unload(&mut self) {
        self.loaded = false;
    }
    fn is_loaded(&self) -> bool {
        self.loaded
    }
    fn generate(
        &mut self,
        req: &GenerateRequest,
        _: &AtomicBool,
        _: &mut dyn FnMut(&str),
    ) -> Result<String> {
        assert!(
            req.grammar.is_some(),
            "every request must be grammar-constrained"
        );
        self.prompts.lock().unwrap().push(req.prompt.clone());
        Ok(if self.replies.is_empty() {
            r#"{"final":"out of script"}"#.into()
        } else {
            self.replies.remove(0)
        })
    }
}

fn worker(replies: &[&str]) -> (ModelWorker, Arc<Mutex<Vec<String>>>) {
    let prompts = Arc::new(Mutex::new(vec![]));
    let w = ModelWorker::spawn(
        Scripted {
            replies: replies.iter().map(|s| s.to_string()).collect(),
            prompts: prompts.clone(),
            loaded: false,
        },
        Duration::from_secs(60),
    );
    (w, prompts)
}

const POSTING_TEXT: &str = "Acme Analytics is hiring a Data Analyst. Fully remote in the US. \
    Pay: $85,000 to $105,000 per year. SQL and Tableau required.";

const EXTRACTED: &str = r#"{"title":"Data Analyst","company":"Acme Analytics","location":null,"remote":"remote","salary_min":85000,"salary_max":105000,"skills":["SQL","Tableau"]}"#;

/// Extract a posting with the task framework and store it, as the search pipeline will.
async fn ingest_posting(db: &Db, replies: &[&str]) -> (i64, u32) {
    let (w, _) = worker(replies);
    let out = task::run::<Value>(
        &w,
        &EXTRACT_POSTING,
        &[("posting", POSTING_TEXT)],
        Priority::Background,
    )
    .await
    .unwrap();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO posting(source, url, title, body, extracted, extracted_with) \
         VALUES ('test', 'https://example.com/jobs/1', ?, ?, ?, ?) RETURNING id",
    )
    .bind(out.value["title"].as_str().unwrap())
    .bind(POSTING_TEXT)
    .bind(out.value.to_string())
    .bind(format!("{}@{}", out.task_id, out.version))
    .fetch_one(db.pool())
    .await
    .unwrap();
    (id, out.attempts)
}

fn tool_spec(name: &'static str, risk: Risk, untrusted: bool) -> ToolSpec {
    ToolSpec {
        name,
        description: "integration test tool.",
        risk,
        input_schema: json!({"type":"object","required":["posting_id"],"properties":{"posting_id":{"type":"integer"}}}),
        action: (risk == Risk::Act).then_some("apply.submit"),
        untrusted_output: untrusted,
    }
}

/// Tools backed by the real database: a read of a posting, and an "apply" that creates an
/// application row with its first status event.
fn registry(db: &Db, applied: Arc<Mutex<Vec<i64>>>) -> Registry {
    let mut r = Registry::new();
    let read_db = db.clone();
    r.register(FnTool::new(
        tool_spec("search.get_posting", Risk::Read, true),
        move |v| -> ToolFuture {
            let db = read_db.clone();
            Box::pin(async move {
                let id = v["posting_id"].as_i64().unwrap();
                let (title, extracted): (String, String) =
                    sqlx::query_as("SELECT title, extracted FROM posting WHERE id = ?")
                        .bind(id)
                        .fetch_one(db.pool())
                        .await
                        .map_err(|e| Error::Database(e.to_string()))?;
                Ok(json!({"title": title, "extracted": serde_json::from_str::<Value>(&extracted).unwrap()}))
            })
        },
    ))
    .unwrap();
    let write_db = db.clone();
    r.register(FnTool::new(
        tool_spec("apply.submit", Risk::Act, false),
        move |v| -> ToolFuture {
            let db = write_db.clone();
            let applied = applied.clone();
            Box::pin(async move {
                let id = v["posting_id"].as_i64().unwrap();
                let app: i64 = sqlx::query_scalar(
                    "INSERT INTO application(posting_id, status) VALUES (?, 'sent') RETURNING id",
                )
                .bind(id)
                .fetch_one(db.pool())
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
                sqlx::query("INSERT INTO status_event(application_id, from_status, to_status, cause) VALUES (?, 'draft', 'sent', 'agent')")
                    .bind(app)
                    .execute(db.pool())
                    .await
                    .map_err(|e| Error::Database(e.to_string()))?;
                applied.lock().unwrap().push(app);
                Ok(json!({"application_id": app}))
            })
        },
    ))
    .unwrap();
    r
}

fn profile() -> AgentProfile {
    AgentProfile::new(
        "apply",
        "You help the user apply.",
        vec!["search.*", "apply.*"],
    )
}

async fn run_agent(w: &ModelWorker, reg: &Registry, policy: &Policy) -> RunResult {
    Harness {
        worker: w,
        registry: reg,
        policy,
        log: &NoLog,
    }
    .run(
        &profile(),
        "Apply to posting 1",
        RunContext::default(),
        &Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap()
}

async fn count(db: &Db, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(db.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn posting_to_queued_application_to_approval() {
    let db = Db::open_in_memory().await.unwrap();
    let (posting_id, attempts) = ingest_posting(&db, &[EXTRACTED]).await;
    assert_eq!(attempts, 1);
    let stored: String = sqlx::query_scalar("SELECT extracted_with FROM posting WHERE id = ?")
        .bind(posting_id)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(stored, "extract_posting@1");

    let applied = Arc::new(Mutex::new(vec![]));
    let reg = registry(&db, applied.clone());
    let (w, prompts) = worker(&[
        r#"{"tool":"search.get_posting","args":{"posting_id":1}}"#,
        r#"{"tool":"apply.submit","args":{"posting_id":1}}"#,
        r#"{"final":"Ready for your approval."}"#,
    ]);
    // Even with the action set to Auto, the posting text is untrusted, so the submit is queued.
    let mut policy = Policy::new();
    policy.set_global("apply.submit", Mode::Auto);
    let result = run_agent(&w, &reg, &policy).await;

    assert_eq!(
        result.outcome,
        Outcome::Final("Ready for your approval.".into())
    );
    assert_eq!(result.queued.len(), 1);
    assert!(
        applied.lock().unwrap().is_empty(),
        "nothing is submitted before approval"
    );
    // The posting data reached the model in the next prompt.
    assert!(prompts.lock().unwrap()[1].contains("Data Analyst"));

    let run_id = approvals::save_run(&db, "apply", "chat", &result)
        .await
        .unwrap();
    assert_eq!(count(&db, "agent_step").await, 2);
    assert_eq!(count(&db, "application").await, 0);
    assert_eq!(approvals::pending_count(&db).await.unwrap(), 1);
    let proposal = approvals::pending(&db).await.unwrap().remove(0);
    assert_eq!(proposal.run_id, Some(run_id));
    assert_eq!(proposal.payload["tool"], "apply.submit");

    let out = approvals::approve_and_run(&db, &reg, proposal.id)
        .await
        .unwrap();
    assert_eq!(out["application_id"], 1);
    assert_eq!(*applied.lock().unwrap(), vec![1]);
    assert_eq!(count(&db, "application").await, 1);
    assert_eq!(count(&db, "status_event").await, 1);
    assert_eq!(approvals::pending_count(&db).await.unwrap(), 0);
    // A double click cannot submit twice.
    assert!(approvals::approve_and_run(&db, &reg, proposal.id)
        .await
        .is_err());
    assert_eq!(count(&db, "application").await, 1);
}

#[tokio::test]
async fn rejected_proposal_never_runs_and_cannot_be_approved_later() {
    let db = Db::open_in_memory().await.unwrap();
    ingest_posting(&db, &[EXTRACTED]).await;
    let applied = Arc::new(Mutex::new(vec![]));
    let reg = registry(&db, applied.clone());
    let (w, _) = worker(&[
        r#"{"tool":"apply.submit","args":{"posting_id":1}}"#,
        r#"{"final":"Queued."}"#,
    ]);
    let result = run_agent(&w, &reg, &Policy::new()).await;
    approvals::save_run(&db, "apply", "chat", &result)
        .await
        .unwrap();
    let id = approvals::pending(&db).await.unwrap()[0].id;

    assert_eq!(approvals::reject(&db, id).await.unwrap().state, "rejected");
    assert!(approvals::approve_and_run(&db, &reg, id).await.is_err());
    assert!(applied.lock().unwrap().is_empty());
    assert_eq!(count(&db, "application").await, 0);
}

#[tokio::test]
async fn auto_mode_runs_the_submit_when_no_untrusted_text_was_read() {
    let db = Db::open_in_memory().await.unwrap();
    ingest_posting(&db, &[EXTRACTED]).await;
    let applied = Arc::new(Mutex::new(vec![]));
    let reg = registry(&db, applied.clone());
    let (w, _) = worker(&[
        r#"{"tool":"apply.submit","args":{"posting_id":1}}"#,
        r#"{"final":"Submitted."}"#,
    ]);
    let mut policy = Policy::new();
    policy.set_global("apply.submit", Mode::Auto);
    let result = run_agent(&w, &reg, &policy).await;
    assert!(result.queued.is_empty());
    assert_eq!(*applied.lock().unwrap(), vec![1]);
    approvals::save_run(&db, "apply", "chat", &result)
        .await
        .unwrap();
    assert_eq!(approvals::pending_count(&db).await.unwrap(), 0);
    assert_eq!(count(&db, "agent_run").await, 1);
}

#[tokio::test]
async fn extraction_retries_once_with_the_error_then_stores_the_result() {
    // First answer breaks the schema (remote is not an allowed value); the second is right.
    let bad = EXTRACTED.replace("\"remote\":\"remote\"", "\"remote\":\"sometimes\"");
    let (w, prompts) = worker(&[&bad, EXTRACTED]);
    let out = task::run::<Value>(
        &w,
        &EXTRACT_POSTING,
        &[("posting", POSTING_TEXT)],
        Priority::Interactive,
    )
    .await
    .unwrap();
    assert_eq!(out.attempts, 2);
    assert_eq!(out.value["salary_min"], 85000);
    assert!(
        prompts.lock().unwrap()[1].contains("rejected"),
        "the retry prompt carries the problem"
    );
}

#[tokio::test]
async fn extraction_that_keeps_failing_is_an_error_not_a_row() {
    let (w, _) = worker(&["not json", "still not json"]);
    let err = task::run::<Value>(
        &w,
        &EXTRACT_POSTING,
        &[("posting", POSTING_TEXT)],
        Priority::Background,
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("failed after"));
}

#[tokio::test]
async fn proposals_and_runs_survive_closing_and_reopening_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ceevee.db");
    {
        let db = Db::open(&path).await.unwrap();
        ingest_posting(&db, &[EXTRACTED]).await;
        let reg = registry(&db, Arc::default());
        let (w, _) = worker(&[
            r#"{"tool":"apply.submit","args":{"posting_id":1}}"#,
            r#"{"final":"Queued."}"#,
        ]);
        let result = run_agent(&w, &reg, &Policy::new()).await;
        approvals::save_run(&db, "apply", "tray", &result)
            .await
            .unwrap();
        db.set_setting("notify.new_jobs", "off").await.unwrap();
    }
    let db = Db::open(&path).await.unwrap();
    assert_eq!(approvals::pending_count(&db).await.unwrap(), 1);
    assert_eq!(count(&db, "posting").await, 1);
    assert_eq!(
        db.get_setting("notify.new_jobs").await.unwrap().as_deref(),
        Some("off")
    );
    // Approving after a restart still runs the tool through a fresh registry.
    let applied = Arc::new(Mutex::new(vec![]));
    let reg = registry(&db, applied.clone());
    let id = approvals::pending(&db).await.unwrap()[0].id;
    approvals::approve_and_run(&db, &reg, id).await.unwrap();
    assert_eq!(applied.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn expired_proposals_cannot_be_approved() {
    let db = Db::open_in_memory().await.unwrap();
    let id = approvals::propose(&db, None, "profile_change", &json!({"field": "phone"}))
        .await
        .unwrap();
    sqlx::query("UPDATE proposal SET created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now','-30 days')")
        .execute(db.pool())
        .await
        .unwrap();
    assert_eq!(approvals::expire_older_than(&db, 14).await.unwrap(), 1);
    assert!(approvals::approve_only(&db, id).await.is_err());
    assert_eq!(approvals::pending_count(&db).await.unwrap(), 0);
}

#[tokio::test]
async fn scheduler_publishes_due_tasks_and_respects_pause_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ceevee.db");
    let tasks = vec![Task {
        id: "search.daily".into(),
        schedule: Schedule::Daily { hour: 7, minute: 0 },
    }];
    let at = |d, h, m| {
        Local
            .with_ymd_and_hms(2026, 11, d, h, m, 0)
            .earliest()
            .unwrap()
    };
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();

    let db = Db::open(&path).await.unwrap();
    // First sight of a daily task only records it; nothing fires on a fresh install.
    assert!(scheduler::tick(&db, &bus, &tasks, at(10, 12, 0))
        .await
        .is_empty());
    // The app is closed, then reopened two days later: the missed slot fires once.
    drop(db);
    let db = Db::open(&path).await.unwrap();
    let due = scheduler::tick(&db, &bus, &tasks, at(12, 8, 0)).await;
    assert_eq!(due, vec!["search.daily"]);
    assert_eq!(
        rx.recv().await.unwrap(),
        Event::TaskDue {
            task: "search.daily".into()
        }
    );
    assert!(scheduler::tick(&db, &bus, &tasks, at(12, 8, 5))
        .await
        .is_empty());

    // Paused: the next day's slot is held back, then fires after the pause ends.
    let until = at(13, 12, 0);
    db.set_setting("background.paused_until", &until.to_rfc3339())
        .await
        .unwrap();
    assert!(scheduler::tick(&db, &bus, &tasks, at(13, 8, 0))
        .await
        .is_empty());
    assert_eq!(
        scheduler::tick(&db, &bus, &tasks, at(13, 12, 30)).await,
        vec!["search.daily"]
    );
}
