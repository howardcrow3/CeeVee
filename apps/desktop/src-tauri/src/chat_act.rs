//! Chat that can act. The person's message goes to the agent harness with a short list of tools.
//! Reading and reversible changes (adding or switching off a job type) run at once; anything
//! the policy queues becomes a proposal in Applications, Waiting on you.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use ceevee_agent::policy::Policy;
use ceevee_agent::runner::{
    AgentProfile, Harness, NoLog, Outcome, RunContext, RunResult, StepResult,
};
use ceevee_agent::tools::Registry;
use serde::Serialize;
use specta::Type;

use crate::models::{worker_for, Engine};

const PROMPT: &str = "You are CeeVee, a private assistant that helps one person with their job \
search. Use the tools to look things up or to add or switch off a kind of job. Do only what the \
person asked. When you are done, say in one or two plain sentences what you found or did.";

/// What the assistant can use from chat.
const ALLOW: &[&str] = &["search.list_jobs", "search.get_posting", "jobtypes.*"];

#[derive(Debug, Clone, Serialize, Type)]
pub struct ChatActResult {
    /// What CeeVee says back.
    pub reply: String,
    /// Plain sentences for each change that was made.
    pub did: Vec<String>,
    /// Changes that now wait for the person's approval.
    pub waiting: u32,
}

/// A plain sentence for one finished step, or `None` for a step that only looked something up.
pub fn describe(tool: &str, args: &serde_json::Value) -> Option<String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("a job type");
    match tool {
        "jobtypes.create" => Some(format!("Added the job type {name}")),
        "jobtypes.set_active" => {
            let on = args.get("active").and_then(|v| v.as_bool()).unwrap_or(true);
            Some(format!(
                "Turned a job type {}",
                if on { "on" } else { "off" }
            ))
        }
        _ => None,
    }
}

/// Turn a finished run into what the person sees.
pub fn summarize(result: &RunResult) -> ChatActResult {
    let did = result
        .steps
        .iter()
        .filter(|s| matches!(s.result, StepResult::Ran(_)))
        .filter_map(|s| describe(&s.tool, &s.args))
        .collect();
    let reply = match &result.outcome {
        Outcome::Final(text) if !text.trim().is_empty() => text.trim().to_string(),
        Outcome::Final(_) => "Done.".to_string(),
        Outcome::StepLimit => {
            "I ran out of steps before I finished. Try a smaller request.".to_string()
        }
        Outcome::Cancelled => "Stopped.".to_string(),
    };
    ChatActResult {
        reply,
        did,
        waiting: u32::try_from(result.queued.len()).unwrap_or(u32::MAX),
    }
}

/// Run one chat message through the agent and return what it said and did.
#[tauri::command]
#[specta::specta]
pub async fn chat_act(
    app: tauri::AppHandle,
    lic: tauri::State<'_, Arc<crate::license_cmd::Service>>,
    engine: tauri::State<'_, Arc<Engine>>,
    db: tauri::State<'_, ceevee_db::Db>,
    registry: tauri::State<'_, Registry>,
    message: String,
) -> Result<ChatActResult, String> {
    crate::license_cmd::require_write(&lic).await?;
    let worker = worker_for(&app, &engine)?;
    let policy = Policy::new();
    let harness = Harness {
        worker: &worker,
        registry: &registry,
        policy: &policy,
        log: &NoLog,
    };
    let profile = AgentProfile::new("Assistant", PROMPT, ALLOW.to_vec());
    let ctx = RunContext {
        category: None,
        user_initiated: false,
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let result = harness
        .run(&profile, &message, ctx, &cancel)
        .await
        .map_err(|e| e.to_string())?;
    ceevee_agent::approvals::save_run(&db, "assistant", "chat", &result)
        .await
        .map_err(|e| e.to_string())?;
    Ok(summarize(&result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ceevee_agent::runner::{QueuedCall, StepRecord};
    use ceevee_agent::tools::Risk;
    use serde_json::json;

    fn step(tool: &str, args: serde_json::Value, result: StepResult) -> StepRecord {
        StepRecord {
            index: 1,
            tool: tool.into(),
            args,
            risk: Risk::Draft,
            result,
        }
    }

    #[test]
    fn says_what_was_done_and_what_waits() {
        let r = RunResult {
            outcome: Outcome::Final("  Added it.  ".into()),
            steps: vec![
                step("search.list_jobs", json!({}), StepResult::Ran(json!([]))),
                step(
                    "jobtypes.create",
                    json!({"name": "Product"}),
                    StepResult::Ran(json!({})),
                ),
                step(
                    "jobtypes.create",
                    json!({"name": "Nope"}),
                    StepResult::Failed("x".into()),
                ),
            ],
            queued: vec![QueuedCall {
                tool: "apply.submit".into(),
                args: json!({}),
                reason: "needs approval".into(),
            }],
        };
        let s = summarize(&r);
        assert_eq!(s.reply, "Added it.");
        assert_eq!(s.did, vec!["Added the job type Product".to_string()]);
        assert_eq!(s.waiting, 1);
    }

    #[test]
    fn step_limit_and_cancel_have_plain_replies() {
        let mk = |outcome| RunResult {
            outcome,
            steps: vec![],
            queued: vec![],
        };
        assert!(summarize(&mk(Outcome::StepLimit))
            .reply
            .contains("ran out of steps"));
        assert_eq!(summarize(&mk(Outcome::Cancelled)).reply, "Stopped.");
        assert_eq!(summarize(&mk(Outcome::Final(" ".into()))).reply, "Done.");
    }

    #[test]
    fn describes_switching_a_job_type() {
        assert_eq!(
            describe("jobtypes.set_active", &json!({"id": 1, "active": false})).as_deref(),
            Some("Turned a job type off")
        );
        assert_eq!(describe("search.list_jobs", &json!({})), None);
    }
}
