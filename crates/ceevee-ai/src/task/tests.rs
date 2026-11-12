use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;

use super::*;
use crate::backend::LlmBackend;
use serde_json::Value;

/// Fake engine that answers from a script and records every prompt it was given.
struct Scripted {
    answers: Vec<String>,
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
        _cancel: &AtomicBool,
        _on_token: &mut dyn FnMut(&str),
    ) -> Result<String> {
        assert!(req.grammar.is_some(), "tasks always send a grammar");
        self.prompts.lock().unwrap().push(req.prompt.clone());
        Ok(self.answers.remove(0))
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct Posting {
    title: String,
    company: Option<String>,
    remote: String,
    salary_min: Option<u32>,
    skills: Vec<String>,
}

fn worker(answers: &[&str]) -> (ModelWorker, Arc<Mutex<Vec<String>>>) {
    let prompts = Arc::new(Mutex::new(vec![]));
    let w = ModelWorker::spawn(
        Scripted {
            answers: answers.iter().map(|s| s.to_string()).collect(),
            prompts: prompts.clone(),
            loaded: false,
        },
        Duration::from_secs(60),
    );
    (w, prompts)
}

const GOOD: &str = r#"{"title":"Data analyst","company":"Acme","location":null,"remote":"hybrid","salary_min":90000,"salary_max":null,"skills":["SQL","Tableau"]}"#;

#[test]
fn render_fills_and_rejects() {
    assert_eq!(
        render("a {{x}} b {{ y }}", &[("x", "1"), ("y", "2")]).unwrap(),
        "a 1 b 2"
    );
    assert_eq!(render("{{x}}", &[("x", "{{y}}")]).unwrap(), "{{y}}");
    assert!(render("{{x}}", &[]).is_err());
    assert!(render("{{x", &[("x", "1")]).is_err());
}

#[test]
fn built_in_task_is_well_formed() {
    let spec = &EXTRACT_POSTING;
    let g = spec.grammar().unwrap();
    assert!(g.contains("root ::="));
    let schema = spec.schema_value().unwrap();
    assert!(parse_output::<Posting>(&schema, GOOD).is_ok());
    assert!(render(spec.prompt, &[("posting", "text")])
        .unwrap()
        .trim_end()
        .ends_with("text"));
}

#[tokio::test]
async fn returns_typed_value() {
    let (w, prompts) = worker(&[GOOD]);
    let out: TaskOutput<Posting> = run(
        &w,
        &EXTRACT_POSTING,
        &[("posting", "We need a data analyst.")],
        Priority::Background,
    )
    .await
    .unwrap();
    assert_eq!(out.value.title, "Data analyst");
    assert_eq!(out.value.skills, vec!["SQL", "Tableau"]);
    assert_eq!(
        (out.attempts, out.version, out.task_id),
        (1, 1, "extract_posting")
    );
    assert!(prompts.lock().unwrap()[0].contains("We need a data analyst."));
}

#[tokio::test]
async fn retries_once_with_the_error() {
    let bad = GOOD.replace("hybrid", "sometimes");
    let (w, prompts) = worker(&[&bad, GOOD]);
    let out: TaskOutput<Posting> = run(
        &w,
        &EXTRACT_POSTING,
        &[("posting", "p")],
        Priority::Background,
    )
    .await
    .unwrap();
    assert_eq!(out.attempts, 2);
    let seen = prompts.lock().unwrap();
    assert!(seen[1].contains("previous answer was rejected"));
    assert!(seen[1].contains("$.remote"));
}

#[tokio::test]
async fn gives_up_after_two_attempts() {
    let (w, _) = worker(&["not json", "{}"]);
    let err = run::<Posting>(
        &w,
        &EXTRACT_POSTING,
        &[("posting", "p")],
        Priority::Background,
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("failed after 2 attempts"), "{err}");
}

#[test]
fn import_resume_task_compiles_and_accepts_a_sample() {
    let spec = &IMPORT_RESUME;
    assert!(spec.grammar().unwrap().contains("root ::="));
    let schema = spec.schema_value().unwrap();
    let sample = r#"{"name":"Maria Lopez","email":null,"phone":null,"location":"Columbus, OH","summary":null,
        "work":[{"company":"Northfield","title":"Analyst","start":"2021-03","end":null,"current":true,"highlights":["Built dashboards"]}],
        "education":[{"school":"Ohio State","degree":"B.S.","field":"Industrial Engineering","end":"2018"}],
        "skills":["SQL"]}"#;
    assert!(parse_output::<Value>(&schema, sample).is_ok());
    let bad = sample.replace("\"current\":true", "\"current\":\"yes\"");
    assert!(parse_output::<Value>(&schema, &bad)
        .unwrap_err()
        .contains("work[0].current"));
    assert!(render(spec.prompt, &[("resume", "RESUME TEXT")])
        .unwrap()
        .contains("RESUME TEXT"));
}
