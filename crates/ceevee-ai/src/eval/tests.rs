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
    for id in [
        "extract_posting",
        "import_resume",
        "score_match",
        "write_cover_letter",
        "tailor_resume",
    ] {
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
            for key in c.expected.as_object().into_iter().flat_map(|o| o.keys()) {
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

/// Every expected value must itself be a legal answer, so a typo in the data cannot make a case
/// impossible to score 1.0 on.
#[test]
fn expected_values_satisfy_the_task_schema_and_names_are_unique() {
    let minimum = [
        ("extract_posting", 12),
        ("import_resume", 8),
        ("score_match", 8),
        ("write_cover_letter", 4),
        ("tailor_resume", 4),
    ];
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
            assert!(
                !c.expected.is_null() || !c.checks.is_empty(),
                "{id}/{}: a case needs expected values or checks",
                c.name
            );
            for (key, value) in c.expected.as_object().into_iter().flatten() {
                crate::validate::validate(&props[key], value)
                    .unwrap_or_else(|e| panic!("{id}/{}: {key}: {e}", c.name));
            }
            for (name, text) in &c.vars {
                assert!(
                    name == "instructions" || !text.trim().is_empty(),
                    "{id}/{}: variable {name} is empty",
                    c.name
                );
            }
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

fn vars(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn each_check_kind_passes_and_fails_as_described() {
    let v = vars(&[
        ("profile", "Maria earned 85,000 and led 40 depots"),
        ("posting", "2+ years of SQL"),
    ]);
    let out = json!({"letter": "Dear Hiring Team, I led 40 depots and know SQL. Sincerely, Maria",
                     "skills": ["SQL", "Excel"], "work": [{"company": "Maria"}]});
    let ok = |c: Check| c.passes(&v, &out);
    assert!(ok(Check::Contains {
        field: "letter".into(),
        text: "SINCERELY,".into()
    }));
    assert!(!ok(Check::Contains {
        field: "letter".into(),
        text: "passionate".into()
    }));
    assert!(ok(Check::NotContains {
        field: "letter".into(),
        text: "passionate".into()
    }));
    assert!(ok(Check::Words {
        field: "letter".into(),
        min: 5,
        max: 20
    }));
    assert!(!ok(Check::Words {
        field: "letter".into(),
        min: 50,
        max: 90
    }));
    assert!(ok(Check::NumbersFrom {
        field: "letter".into(),
        var: "profile+posting".into()
    }));
    let invented = json!({"letter": "I led 45 depots"});
    assert!(!Check::NumbersFrom {
        field: "letter".into(),
        var: "profile".into()
    }
    .passes(&v, &invented));
    assert!(ok(Check::Count {
        field: "skills".into(),
        n: 2
    }));
    assert!(ok(Check::FirstContains {
        field: "skills".into(),
        text: "sql".into()
    }));
    assert!(!ok(Check::ItemsFrom {
        field: "skills".into(),
        var: "profile".into()
    }));
    assert!(ok(Check::CompaniesFrom {
        var: "profile".into()
    }));
}

#[test]
fn thousands_separators_do_not_hide_a_number() {
    assert_eq!(
        numbers("85,000 and 85000 and 2+"),
        ["2", "85000"].iter().map(|s| s.to_string()).collect()
    );
}

#[test]
fn checks_and_expected_values_combine_by_their_mean() {
    let case: Case = serde_json::from_value(json!({
        "name": "n", "vars": {"a": "x"},
        "expected": {"k": 1},
        "checks": [{"kind": "count", "field": "l", "n": 1}, {"kind": "count", "field": "l", "n": 2}]
    }))
    .unwrap();
    // expected right (1.0), one of two checks right (0.5): mean 0.75.
    let (score, fields) = score_any(&case, &json!({"k": 1, "l": [1]}));
    assert!((score - 0.75).abs() < 1e-9, "{score}");
    assert_eq!(fields.len(), 3);
}

/// An answer that satisfies every rule scores 1.0: tailored resumes are the input with the posting's
/// skill moved to the front, so the oracle can be built from the case itself.
#[test]
fn a_faithful_tailoring_passes_every_check() {
    let cases = parse_cases(builtin_cases("tailor_resume").unwrap()).unwrap();
    for c in &cases {
        let mut resume: Value = serde_json::from_str(&c.vars["resume"]).unwrap();
        let first = c
            .checks
            .iter()
            .find_map(|k| match k {
                Check::FirstContains { text, .. } => Some(text.to_lowercase()),
                _ => None,
            })
            .unwrap_or_default();
        let skills = resume["skills"].as_array_mut().unwrap();
        if let Some(i) = skills
            .iter()
            .position(|s| s.as_str().unwrap().to_lowercase().contains(&first))
        {
            let s = skills.remove(i);
            skills.insert(0, s);
        }
        let (score, fields) = score_any(c, &resume);
        assert_eq!(score, 1.0, "{}: {fields:?}", c.name);
    }
}

#[test]
fn a_plain_letter_passes_the_first_letter_case() {
    let cases = parse_cases(builtin_cases("write_cover_letter").unwrap()).unwrap();
    let letter = "Dear Hiring Team,\n\nI am writing about the Data Analyst role at Acme Analytics. \
        The posting asks for SQL, Tableau dashboards and clear written communication, which is the work I do every week. \
        I like that the role is remote and focused on reporting people actually read.\n\n\
        At Northfield Logistics I am a Senior Analyst, where I built Tableau dashboards and SQL reporting for 40 depots, \
        and I cut report preparation from two days to four hours. Before that I was an Analyst at Harbor Freight Co. from 2018 to 2021, \
        writing SQL reports. I hold a B.S. in Industrial Engineering from Ohio State University and also use Python and Excel.\n\n\
        In both roles my job was to turn raw delivery data into reports that managers could act on the same day, \
        and to explain the numbers plainly to people who do not work with data. I would welcome a conversation about how that \
        experience fits your team, and I thank you for your time and for considering my application.\n\n\
        Sincerely,\nMaria Lopez";
    let (score, fields) = score_any(&cases[0], &json!({"letter": letter}));
    assert_eq!(score, 1.0, "{fields:?}");
    // A letter with a cliche and an invented number loses points.
    let bad = format!("{letter} I am passionate and managed 400 depots!");
    assert!(score_any(&cases[0], &json!({"letter": bad})).0 < 1.0);
}
