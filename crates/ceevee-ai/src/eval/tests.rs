use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::Mutex;

use serde_json::json;

use super::*;
use crate::backend::{GenerateRequest, LlmBackend};
use crate::task::EXTRACT_POSTING;

#[test]
fn field_scores() {
    assert_eq!(score_field(&json!("Remote"), &json!(" remote ")), 1.0);
    assert_eq!(score_field(&json!(85000), &json!(90000)), 0.0);
    assert_eq!(score_field(&json!(null), &json!(null)), 1.0);
    assert_eq!(score_field(&json!([]), &json!([])), 1.0);
    assert_eq!(
        score_field(&json!(["SQL", "Tableau"]), &json!(["sql"])),
        2.0 / 3.0
    );
    assert_eq!(score_field(&json!(["a"]), &json!(["b"])), 0.0);
}

#[test]
fn case_score_is_the_mean_and_missing_fields_score_zero() {
    let (s, f) = score_case(&json!({"a": 1, "b": 2}), &json!({"a": 1}));
    assert_eq!(s, 0.5);
    assert_eq!(f["b"], 0.0);
}

#[test]
fn built_in_cases_parse_and_use_fields_the_schema_defines() {
    for id in ["extract_posting", "import_resume"] {
        let spec = builtin_task(id).unwrap();
        let schema = spec.schema_value().unwrap();
        let props = schema["properties"].as_object().unwrap();
        let cases = parse_cases(builtin_cases(id).unwrap()).unwrap();
        assert!(cases.len() >= 2, "{id}");
        for c in &cases {
            assert!(c.vars.len() == 1, "{id}/{}", c.name);
            for key in c.expected.as_object().unwrap().keys() {
                assert!(
                    props.contains_key(key),
                    "{id}/{}: unknown field {key}",
                    c.name
                );
            }
        }
    }
    assert!(builtin_cases("nope").is_none() && builtin_task("nope").is_none());
}

struct Scripted(Arc<Mutex<Vec<String>>>, bool);
impl LlmBackend for Scripted {
    fn load(&mut self) -> ceevee_core::Result<()> {
        self.1 = true;
        Ok(())
    }
    fn unload(&mut self) {
        self.1 = false;
    }
    fn is_loaded(&self) -> bool {
        self.1
    }
    fn generate(
        &mut self,
        _: &GenerateRequest,
        _: &AtomicBool,
        _: &mut dyn FnMut(&str),
    ) -> ceevee_core::Result<String> {
        Ok(self.0.lock().unwrap().remove(0))
    }
}

#[tokio::test]
async fn runs_cases_and_reports() {
    let cases = parse_cases(builtin_cases("extract_posting").unwrap()).unwrap();
    let cases = &cases[..3];
    // Case 1 answered perfectly, case 2 wrong in one field, case 3 never valid.
    let first = cases[0].expected.to_string();
    let mut second = cases[1].expected.clone();
    second["remote"] = json!("hybrid");
    let replies = vec![first, second.to_string(), "bad".into(), "bad".into()];
    let w = ModelWorker::spawn(
        Scripted(Arc::new(Mutex::new(replies)), false),
        std::time::Duration::from_secs(60),
    );
    let report = run_eval(&w, &EXTRACT_POSTING, cases).await;
    assert_eq!(report.cases[0].score, Some(1.0));
    let s1 = report.cases[1].score.unwrap();
    assert!((s1 - 6.0 / 7.0).abs() < 1e-9, "{s1}");
    assert_eq!(report.cases[2].score, None);
    assert_eq!(report.failures(), 1);
    let text = report.summary();
    assert!(
        text.contains("extract_posting v1")
            && text.contains("FAIL")
            && text.contains("weak: remote=0.00"),
        "{text}"
    );
}

/// Every expected value must itself be a legal answer, so a typo in the data cannot make a case
/// impossible to score 1.0 on.
#[test]
fn expected_values_satisfy_the_task_schema_and_names_are_unique() {
    let minimum = [("extract_posting", 12), ("import_resume", 8)];
    for (id, min) in minimum {
        let schema = builtin_task(id).unwrap().schema_value().unwrap();
        let props = schema["properties"].as_object().unwrap();
        let cases = parse_cases(builtin_cases(id).unwrap()).unwrap();
        assert!(
            cases.len() >= min,
            "{id} has {} cases, want at least {min}",
            cases.len()
        );
        let mut names = std::collections::BTreeSet::new();
        for c in &cases {
            assert!(
                names.insert(c.name.clone()),
                "{id}: duplicate case name {}",
                c.name
            );
            for (key, value) in c.expected.as_object().unwrap() {
                crate::validate::validate(&props[key], value)
                    .unwrap_or_else(|e| panic!("{id}/{}: {key}: {e}", c.name));
            }
            let text = c.vars.values().next().unwrap();
            assert!(!text.trim().is_empty(), "{id}/{}", c.name);
        }
    }
}

/// A model that answers every case exactly as expected scores 1.0; this guards the scorer and
/// the case files together.
#[tokio::test]
async fn a_perfect_model_scores_one_on_every_extract_posting_case() {
    let cases = parse_cases(builtin_cases("extract_posting").unwrap()).unwrap();
    let replies: Vec<String> = cases.iter().map(|c| c.expected.to_string()).collect();
    let w = ModelWorker::spawn(
        Scripted(Arc::new(Mutex::new(replies)), false),
        std::time::Duration::from_secs(60),
    );
    let report = run_eval(&w, &EXTRACT_POSTING, &cases).await;
    assert_eq!(report.failures(), 0, "{}", report.summary());
    assert_eq!(report.mean_score(), 1.0);
}

#[tokio::test]
async fn a_spent_budget_skips_the_remaining_cases_and_says_so() {
    let cases = parse_cases(builtin_cases("extract_posting").unwrap()).unwrap();
    let w = ModelWorker::spawn(
        Scripted(Arc::new(Mutex::new(vec![])), false),
        std::time::Duration::from_secs(60),
    );
    let report = run_eval_within(
        &w,
        &EXTRACT_POSTING,
        &cases,
        Some(std::time::Duration::ZERO),
    )
    .await;
    assert!(report.cases.is_empty());
    assert_eq!(report.skipped, cases.len());
    assert!(report.summary().contains("skipped"));
}

#[tokio::test]
async fn report_json_carries_model_scores_and_skips() {
    let cases = parse_cases(builtin_cases("extract_posting").unwrap()).unwrap();
    let w = ModelWorker::spawn(
        Scripted(
            Arc::new(Mutex::new(vec![cases[0].expected.to_string()])),
            false,
        ),
        std::time::Duration::from_secs(60),
    );
    let one = &cases[..1];
    let report = run_eval(&w, &EXTRACT_POSTING, one).await;
    let j = report.to_json("qwen3-0.6b");
    assert_eq!(j["model"], "qwen3-0.6b");
    assert_eq!(j["task"], "extract_posting");
    assert_eq!(j["mean_score"], 1.0);
    assert_eq!(j["total"], 1);
    assert_eq!(j["cases"][0]["name"], one[0].name);
}
