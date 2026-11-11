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
    for id in ["extract_posting", "import_resume", "score_match"] {
        let spec = builtin_task(id).unwrap();
        let schema = spec.schema_value().unwrap();
        let props = schema["properties"].as_object().unwrap();
        let cases = parse_cases(builtin_cases(id).unwrap()).unwrap();
        assert!(cases.len() >= 2, "{id}");
        for c in &cases {
            let vars: Vec<(&str, &str)> = c
                .vars
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            crate::task::render(spec.prompt, &vars)
                .unwrap_or_else(|e| panic!("{id}/{}: {e}", c.name));
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
    // Case 1 answered perfectly, case 2 wrong in one field, case 3 never valid.
    let first = cases[0].expected.to_string();
    let mut second = cases[1].expected.clone();
    second["remote"] = json!("hybrid");
    let replies = vec![first, second.to_string(), "bad".into(), "bad".into()];
    let w = ModelWorker::spawn(
        Scripted(Arc::new(Mutex::new(replies)), false),
        std::time::Duration::from_secs(60),
    );
    let report = run_eval(&w, &EXTRACT_POSTING, &cases).await;
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

#[test]
fn tolerance_makes_a_near_number_right() {
    let tol = [("score".to_string(), 10.0)].into_iter().collect();
    assert_eq!(
        score_case_with(&json!({"score": 85}), &json!({"score": 92}), &tol).0,
        1.0
    );
    assert_eq!(
        score_case_with(&json!({"score": 85}), &json!({"score": 60}), &tol).0,
        0.0
    );
    // Without a tolerance the number must match exactly.
    assert_eq!(
        score_case(&json!({"score": 85}), &json!({"score": 86})).0,
        0.0
    );
}
