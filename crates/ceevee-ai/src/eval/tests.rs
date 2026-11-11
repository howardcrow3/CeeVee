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
fn built_in_cases_parse_and_match_the_schema() {
    let cases = parse_cases(builtin_cases("extract_posting").unwrap()).unwrap();
    assert!(cases.len() >= 3);
    let schema = EXTRACT_POSTING.schema_value().unwrap();
    for c in &cases {
        crate::validate::validate(&schema, &c.expected)
            .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert!(c.vars.contains_key("posting"));
    }
    assert!(builtin_cases("nope").is_none());
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
