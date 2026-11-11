use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use ceevee_ai::LlmBackend;
use serde_json::json;

use super::*;
use crate::policy::Mode;
use crate::tools::{FnTool, Risk, ToolFuture};

/// Fake engine that replies from a script and keeps the prompts it saw.
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
        assert!(req.grammar.is_some());
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

fn spec(name: &'static str, risk: Risk, untrusted: bool) -> ToolSpec {
    ToolSpec {
        name,
        description: "test tool.",
        risk,
        input_schema: json!({"type":"object","required":["id"],"properties":{"id":{"type":"integer"}}}),
        action: (risk == Risk::Act).then_some("apply.submit"),
        untrusted_output: untrusted,
    }
}

/// A registry with a read tool, an act tool, and counters showing which ran.
fn registry(ran: Arc<Mutex<Vec<String>>>, untrusted_read: bool) -> Registry {
    let mut r = Registry::new();
    for (name, risk, untrusted) in [
        ("search.get_posting", Risk::Read, untrusted_read),
        ("apply.submit", Risk::Act, false),
    ] {
        let ran = ran.clone();
        r.register(FnTool::new(
            spec(name, risk, untrusted),
            move |v| -> ToolFuture {
                let ran = ran.clone();
                Box::pin(async move {
                    ran.lock().unwrap().push(name.to_string());
                    Ok(json!({"title": "Data analyst", "echo": v}))
                })
            },
        ))
        .unwrap();
    }
    r
}

fn profile() -> AgentProfile {
    AgentProfile::new("test", "You are a test agent.", vec!["search.*", "apply.*"])
}

async fn run_simple(
    w: &ModelWorker,
    log: &dyn RunLog,
    cancel: &Arc<AtomicBool>,
) -> Result<RunResult> {
    let reg = registry(Arc::default(), false);
    let pol = Policy::new();
    Harness {
        worker: w,
        registry: &reg,
        policy: &pol,
        log,
    }
    .run(&profile(), "g", RunContext::default(), cancel)
    .await
}

async fn go(
    w: &ModelWorker,
    r: &Registry,
    p: &Policy,
    prof: &AgentProfile,
    ctx: RunContext,
) -> RunResult {
    Harness {
        worker: w,
        registry: r,
        policy: p,
        log: &NoLog,
    }
    .run(prof, "do the thing", ctx, &Arc::new(AtomicBool::new(false)))
    .await
    .unwrap()
}

#[test]
fn step_schema_lists_only_allowed_tools_and_compiles() {
    let s1 = spec("search.get_posting", Risk::Read, false);
    let schema = step_schema(&[&s1]);
    let g = json_schema_to_gbnf(&schema).unwrap();
    assert!(g.contains("search.get_posting"));
    assert!(!g.contains("apply.submit"));
}

#[tokio::test]
async fn calls_a_tool_then_answers_with_the_result_in_the_next_prompt() {
    let ran = Arc::new(Mutex::new(vec![]));
    let (w, prompts) = worker(&[
        r#"{"tool":"search.get_posting","args":{"id":4}}"#,
        r#"{"final":"Found it."}"#,
    ]);
    let out = go(
        &w,
        &registry(ran.clone(), false),
        &Policy::new(),
        &profile(),
        RunContext::default(),
    )
    .await;
    assert_eq!(out.outcome, Outcome::Final("Found it.".into()));
    assert_eq!(out.steps.len(), 1);
    assert_eq!(*ran.lock().unwrap(), vec!["search.get_posting"]);
    let seen = prompts.lock().unwrap();
    assert!(seen[0].contains("No steps yet") && seen[0].contains("Goal: do the thing"));
    assert!(seen[1].contains("1. search.get_posting") && seen[1].contains("Data analyst"));
}

#[tokio::test]
async fn act_tool_is_queued_not_run_by_default() {
    let ran = Arc::new(Mutex::new(vec![]));
    let (w, prompts) = worker(&[
        r#"{"tool":"apply.submit","args":{"id":1}}"#,
        r#"{"final":"Waiting for you."}"#,
    ]);
    let out = go(
        &w,
        &registry(ran.clone(), false),
        &Policy::new(),
        &profile(),
        RunContext::default(),
    )
    .await;
    assert!(ran.lock().unwrap().is_empty());
    assert_eq!(out.queued.len(), 1);
    assert_eq!(out.queued[0].tool, "apply.submit");
    assert!(matches!(out.steps[0].result, StepResult::Queued(_)));
    assert!(prompts.lock().unwrap()[1].contains("waiting for the user's approval"));
}

#[tokio::test]
async fn auto_mode_runs_it_but_not_after_untrusted_text() {
    let mut policy = Policy::new();
    policy.set_global("apply.submit", Mode::Auto);

    let ran = Arc::new(Mutex::new(vec![]));
    let (w, _) = worker(&[
        r#"{"tool":"apply.submit","args":{"id":1}}"#,
        r#"{"final":"ok"}"#,
    ]);
    go(
        &w,
        &registry(ran.clone(), false),
        &policy,
        &profile(),
        RunContext::default(),
    )
    .await;
    assert_eq!(*ran.lock().unwrap(), vec!["apply.submit"]);

    let ran = Arc::new(Mutex::new(vec![]));
    let (w, _) = worker(&[
        r#"{"tool":"search.get_posting","args":{"id":1}}"#,
        r#"{"tool":"apply.submit","args":{"id":1}}"#,
        r#"{"final":"ok"}"#,
    ]);
    let out = go(
        &w,
        &registry(ran.clone(), true),
        &policy,
        &profile(),
        RunContext::default(),
    )
    .await;
    assert_eq!(*ran.lock().unwrap(), vec!["search.get_posting"]);
    assert_eq!(out.queued.len(), 1);
}

#[tokio::test]
async fn off_mode_denies() {
    let mut policy = Policy::new();
    policy.set_global("apply.submit", Mode::Off);
    let ran = Arc::new(Mutex::new(vec![]));
    let (w, _) = worker(&[
        r#"{"tool":"apply.submit","args":{"id":1}}"#,
        r#"{"final":"ok"}"#,
    ]);
    let out = go(
        &w,
        &registry(ran.clone(), false),
        &policy,
        &profile(),
        RunContext::default(),
    )
    .await;
    assert!(matches!(out.steps[0].result, StepResult::Denied(_)));
    assert!(ran.lock().unwrap().is_empty());
}

#[tokio::test]
async fn stops_at_the_step_limit() {
    let ran = Arc::new(Mutex::new(vec![]));
    let call = r#"{"tool":"search.get_posting","args":{"id":1}}"#;
    let (w, _) = worker(&[call, call, call, call]);
    let mut prof = profile();
    prof.max_steps = 3;
    let out = go(
        &w,
        &registry(ran.clone(), false),
        &Policy::new(),
        &prof,
        RunContext::default(),
    )
    .await;
    assert_eq!(out.outcome, Outcome::StepLimit);
    assert_eq!(out.steps.len(), 3);
}

#[tokio::test]
async fn bad_arguments_get_one_retry_then_fail() {
    let ran = Arc::new(Mutex::new(vec![]));
    let (w, prompts) = worker(&[
        r#"{"tool":"search.get_posting","args":{"id":"x"}}"#,
        r#"{"tool":"search.get_posting","args":{"id":2}}"#,
        r#"{"final":"ok"}"#,
    ]);
    let out = go(
        &w,
        &registry(ran, false),
        &Policy::new(),
        &profile(),
        RunContext::default(),
    )
    .await;
    assert_eq!(out.steps.len(), 1);
    assert!(prompts.lock().unwrap()[1].contains("previous reply was rejected"));

    let (w, _) = worker(&["nope", "still nope"]);
    let err = run_simple(&w, &NoLog, &Arc::new(AtomicBool::new(false)))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("valid step"));
}

#[tokio::test]
async fn cancel_stops_before_the_next_step() {
    let (w, _) = worker(&[r#"{"final":"never"}"#]);
    let cancel = Arc::new(AtomicBool::new(true));
    let out = run_simple(&w, &NoLog, &cancel).await.unwrap();
    assert_eq!(out.outcome, Outcome::Cancelled);
}

#[tokio::test]
async fn log_receives_each_step() {
    struct Count(Mutex<Vec<String>>);
    impl RunLog for Count {
        fn step(&self, r: &StepRecord) {
            self.0.lock().unwrap().push(r.tool.clone());
        }
    }
    let log = Count(Mutex::new(vec![]));
    let (w, _) = worker(&[
        r#"{"tool":"search.get_posting","args":{"id":1}}"#,
        r#"{"final":"ok"}"#,
    ]);
    run_simple(&w, &log, &Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    assert_eq!(*log.0.lock().unwrap(), vec!["search.get_posting"]);
}
