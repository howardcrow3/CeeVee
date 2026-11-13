//! Eval harness: run a task over scripted cases and score the answers against the expected values
//! (TECHNICAL_PLAN section 5). Use it to compare models and prompt versions before changing a
//! default, and to benchmark a laptop.
//!
//! Scoring is per top-level field, then averaged: scalars match exactly (strings ignoring case and
//! surrounding space), lists of strings score by F1 over their normalized members.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use ceevee_core::Result;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::task::{run, TaskSpec};
use crate::worker::{ModelWorker, Priority};

#[derive(Debug, Clone, Deserialize)]
pub struct Case {
    pub name: String,
    pub vars: std::collections::BTreeMap<String, String>,
    /// Exact answers to compare, field by field. Empty for free-text tasks that use `checks`.
    #[serde(default)]
    pub expected: Value,
    /// Rules the answer must satisfy, for tasks whose output is prose (cover letters) or a
    /// rearranged copy of the input (tailored resumes). Each passing rule scores 1.
    #[serde(default)]
    pub checks: Vec<Check>,
    /// Numeric fields that count as right when within this distance of the expected value (for
    /// example a match score of 88 against an expected 85 with a tolerance of 10).
    #[serde(default)]
    pub tolerance: std::collections::BTreeMap<String, f64>,
}

/// One rule about a task's answer. `field` is a top-level field of the answer.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Check {
    /// The text field contains `text` (ignoring case).
    Contains { field: String, text: String },
    /// The text field does not contain `text` (ignoring case).
    NotContains { field: String, text: String },
    /// The text field has between `min` and `max` words.
    Words {
        field: String,
        min: usize,
        max: usize,
    },
    /// Every number in the text field also appears in the case variable(s) `var` (several joined
    /// with "+"), so the model did not invent or round a figure.
    NumbersFrom { field: String, var: String },
    /// Every string in the list field appears (ignoring case) in the case variable `var`, so
    /// nothing was added.
    ItemsFrom { field: String, var: String },
    /// The list field has exactly `n` items.
    Count { field: String, n: usize },
    /// The list field's first item contains `text` (ignoring case).
    FirstContains { field: String, text: String },
    /// Every `company` in the `work` list appears in the case variable `var`.
    CompaniesFrom { var: String },
}

impl Check {
    fn describe(&self) -> String {
        match self {
            Check::Contains { field, text } => format!("{field} contains \"{text}\""),
            Check::NotContains { field, text } => format!("{field} avoids \"{text}\""),
            Check::Words { field, min, max } => format!("{field} has {min}-{max} words"),
            Check::NumbersFrom { field, var } => format!("{field} numbers come from {var}"),
            Check::ItemsFrom { field, var } => format!("{field} items come from {var}"),
            Check::Count { field, n } => format!("{field} has {n} items"),
            Check::FirstContains { field, text } => format!("{field} starts with \"{text}\""),
            Check::CompaniesFrom { var } => format!("employers come from {var}"),
        }
    }

    fn passes(&self, vars: &std::collections::BTreeMap<String, String>, out: &Value) -> bool {
        let text = |f: &str| {
            out.get(f)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase()
        };
        // `var` may name several variables joined with "+", such as "profile+posting".
        let var = |v: &str| {
            v.split('+')
                .filter_map(|n| vars.get(n.trim()))
                .map(|s| s.to_lowercase())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let list = |f: &str| {
            out.get(f)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        match self {
            Check::Contains { field, text: t } => text(field).contains(&t.to_lowercase()),
            Check::NotContains { field, text: t } => !text(field).contains(&t.to_lowercase()),
            Check::Words { field, min, max } => {
                let n = text(field).split_whitespace().count();
                (*min..=*max).contains(&n)
            }
            Check::NumbersFrom { field, var: v } => {
                let source = var(v);
                numbers(&text(field))
                    .iter()
                    .all(|n| numbers(&source).contains(n))
            }
            Check::ItemsFrom { field, var: v } => {
                let source = var(v);
                list(field).iter().all(|i| {
                    i.as_str()
                        .is_some_and(|s| source.contains(&s.to_lowercase()))
                })
            }
            Check::Count { field, n } => list(field).len() == *n,
            Check::FirstContains { field, text: t } => list(field)
                .first()
                .and_then(Value::as_str)
                .is_some_and(|s| s.to_lowercase().contains(&t.to_lowercase())),
            Check::CompaniesFrom { var: v } => {
                let source = var(v);
                list("work").iter().all(|j| {
                    j.get("company")
                        .and_then(Value::as_str)
                        .is_some_and(|c| source.contains(&c.to_lowercase()))
                })
            }
        }
    }
}

/// The digit runs in a text, with thousands separators removed ("85,000" and "85000" match).
fn numbers(text: &str) -> BTreeSet<String> {
    let cleaned = text.replace(',', "");
    let mut found = BTreeSet::new();
    let mut run = String::new();
    for c in cleaned.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            run.push(c);
        } else if !run.is_empty() {
            found.insert(std::mem::take(&mut run));
        }
    }
    found
}

/// Score a case's checks: the share that pass, and each check's result by description.
pub fn score_checks(
    checks: &[Check],
    vars: &std::collections::BTreeMap<String, String>,
    out: &Value,
) -> (f64, Map<String, Value>) {
    let mut fields = Map::new();
    let mut passed = 0.0;
    for c in checks {
        let ok = c.passes(vars, out);
        passed += f64::from(ok);
        fields.insert(c.describe(), Value::from(f64::from(ok)));
    }
    let mean = if checks.is_empty() {
        1.0
    } else {
        passed / checks.len() as f64
    };
    (mean, fields)
}

pub fn parse_cases(json: &str) -> Result<Vec<Case>> {
    serde_json::from_str(json).map_err(|e| ceevee_core::Error::Invalid(format!("eval cases: {e}")))
}

fn norm(v: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(s.trim().to_lowercase()),
        other => other.clone(),
    }
}

fn members(v: &Value) -> Option<BTreeSet<String>> {
    v.as_array().map(|a| {
        a.iter()
            .map(|x| match norm(x) {
                Value::String(s) => s,
                other => other.to_string(),
            })
            .collect()
    })
}

/// Score one field from 0.0 to 1.0.
pub fn score_field(expected: &Value, actual: &Value) -> f64 {
    if let (Some(e), Some(a)) = (members(expected), members(actual)) {
        if e.is_empty() && a.is_empty() {
            return 1.0;
        }
        let hits = e.intersection(&a).count() as f64;
        if hits == 0.0 {
            return 0.0;
        }
        let (p, r) = (hits / a.len() as f64, hits / e.len() as f64);
        return 2.0 * p * r / (p + r);
    }
    f64::from(norm(expected) == norm(actual))
}

/// Mean field score over the expected object's fields. A field missing from `actual` scores 0.
pub fn score_case(expected: &Value, actual: &Value) -> (f64, Map<String, Value>) {
    score_case_with(expected, actual, &Default::default())
}

/// Like [`score_case`], with per-field numeric tolerances.
pub fn score_case_with(
    expected: &Value,
    actual: &Value,
    tolerance: &std::collections::BTreeMap<String, f64>,
) -> (f64, Map<String, Value>) {
    let empty = Map::new();
    let exp = expected.as_object().unwrap_or(&empty);
    let mut fields = Map::new();
    let mut total = 0.0;
    for (k, ev) in exp {
        let near = |a: &Value| match (ev.as_f64(), a.as_f64(), tolerance.get(k)) {
            (Some(e), Some(x), Some(t)) => Some(f64::from((e - x).abs() <= *t)),
            _ => None,
        };
        let s = actual
            .get(k)
            .map_or(0.0, |av| near(av).unwrap_or_else(|| score_field(ev, av)));
        total += s;
        fields.insert(k.clone(), Value::from(s));
    }
    let mean = if exp.is_empty() {
        1.0
    } else {
        total / exp.len() as f64
    };
    (mean, fields)
}

/// Score a case with its exact answers, its checks, or both (their mean).
fn score_any(case: &Case, out: &Value) -> (f64, Map<String, Value>) {
    let has_expected = case.expected.as_object().is_some_and(|o| !o.is_empty());
    let (exact, mut fields) = score_case_with(&case.expected, out, &case.tolerance);
    if case.checks.is_empty() {
        return (exact, fields);
    }
    let (rules, rule_fields) = score_checks(&case.checks, &case.vars, out);
    fields.extend(rule_fields);
    if has_expected {
        ((exact + rules) / 2.0, fields)
    } else {
        (rules, fields)
    }
}

#[derive(Debug, Clone)]
pub struct CaseReport {
    pub name: String,
    /// `None` when the task failed outright (no valid output after the retry).
    pub score: Option<f64>,
    pub fields: Map<String, Value>,
    pub attempts: u32,
    pub took: Duration,
    /// Length of the model's raw output. With `took` this shows whether a slow case was a slow
    /// machine or a model that rambles on until the token limit.
    pub output_chars: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub task: &'static str,
    pub version: u32,
    pub cases: Vec<CaseReport>,
    /// Cases not run because the time budget ran out.
    pub skipped: usize,
}

impl Report {
    /// Mean score, counting a failed case as 0.
    pub fn mean_score(&self) -> f64 {
        if self.cases.is_empty() {
            return 0.0;
        }
        self.cases
            .iter()
            .map(|c| c.score.unwrap_or(0.0))
            .sum::<f64>()
            / self.cases.len() as f64
    }

    pub fn failures(&self) -> usize {
        self.cases.iter().filter(|c| c.score.is_none()).count()
    }

    pub fn total_time(&self) -> Duration {
        self.cases.iter().map(|c| c.took).sum()
    }

    /// The report as JSON for `scripts/eval-report.mjs`, which compares models.
    pub fn to_json(&self, model: &str) -> Value {
        let cases: Vec<Value> = self
            .cases
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "score": c.score,
                    "fields": c.fields,
                    "attempts": c.attempts,
                    "seconds": c.took.as_secs_f64(),
                    "output_chars": c.output_chars,
                    "error": c.error,
                })
            })
            .collect();
        serde_json::json!({
            "model": model,
            "task": self.task,
            "version": self.version,
            "mean_score": self.mean_score(),
            "failed": self.failures(),
            "skipped": self.skipped,
            "total": self.cases.len() + self.skipped,
            "seconds": self.total_time().as_secs_f64(),
            "cases": cases,
        })
    }

    /// A plain-text summary for the terminal or a pull request.
    pub fn summary(&self) -> String {
        let mut out = format!(
            "{} v{}: mean score {:.2}, {} of {} cases failed outright, {:.1}s total\n",
            self.task,
            self.version,
            self.mean_score(),
            self.failures(),
            self.cases.len(),
            self.total_time().as_secs_f64()
        );
        if self.skipped > 0 {
            out.push_str(&format!(
                "  {} case(s) skipped: time budget used up\n",
                self.skipped
            ));
        }
        for c in &self.cases {
            match (&c.score, &c.error) {
                (Some(s), _) => {
                    let weak: Vec<String> = c
                        .fields
                        .iter()
                        .filter(|(_, v)| v.as_f64().unwrap_or(0.0) < 1.0)
                        .map(|(k, v)| format!("{k}={:.2}", v.as_f64().unwrap_or(0.0)))
                        .collect();
                    out.push_str(&format!(
                        "  {:.2}  {} ({} attempt(s), {:.1}s){}\n",
                        s,
                        c.name,
                        c.attempts,
                        c.took.as_secs_f64(),
                        if weak.is_empty() {
                            String::new()
                        } else {
                            format!("  weak: {}", weak.join(", "))
                        }
                    ));
                }
                (None, e) => out.push_str(&format!(
                    "  FAIL  {}: {}\n",
                    c.name,
                    e.clone().unwrap_or_default()
                )),
            }
        }
        out
    }
}

/// Run every case through the task on the worker and score it.
pub async fn run_eval(worker: &ModelWorker, spec: &TaskSpec, cases: &[Case]) -> Report {
    run_eval_within(worker, spec, cases, None).await
}

/// Like [`run_eval`], but stop starting new cases once `budget` has passed. Slow hosted runners
/// then report the cases they reached instead of hitting a job timeout.
pub async fn run_eval_within(
    worker: &ModelWorker,
    spec: &TaskSpec,
    cases: &[Case],
    budget: Option<Duration>,
) -> Report {
    let begun = Instant::now();
    let mut reports = Vec::new();
    for case in cases {
        if budget.is_some_and(|b| begun.elapsed() >= b) {
            break;
        }
        let vars: Vec<(&str, &str)> = case
            .vars
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let started = Instant::now();
        let outcome = run::<Value>(worker, spec, &vars, Priority::Background).await;
        let took = started.elapsed();
        reports.push(match outcome {
            Ok(out) => {
                let (score, fields) = score_any(case, &out.value);
                CaseReport {
                    name: case.name.clone(),
                    score: Some(score),
                    fields,
                    attempts: out.attempts,
                    took,
                    output_chars: out.raw.chars().count(),
                    error: None,
                }
            }
            Err(e) => CaseReport {
                name: case.name.clone(),
                score: None,
                fields: Map::new(),
                attempts: 0,
                took,
                output_chars: 0,
                error: Some(e.to_string()),
            },
        });
    }
    Report {
        task: spec.id,
        version: spec.version,
        skipped: cases.len() - reports.len(),
        cases: reports,
    }
}

/// The built-in cases for each task, compiled in.
pub fn builtin_cases(task_id: &str) -> Option<&'static str> {
    match task_id {
        "extract_posting" => Some(include_str!("../evals/extract_posting.json")),
        "import_resume" => Some(include_str!("../evals/import_resume.json")),
        "score_match" => Some(include_str!("../evals/score_match.json")),
        "write_cover_letter" => Some(include_str!("../evals/write_cover_letter.json")),
        "tailor_resume" => Some(include_str!("../evals/tailor_resume.json")),
        _ => None,
    }
}

/// Look up a built-in task by id.
pub fn builtin_task(task_id: &str) -> Option<&'static TaskSpec> {
    match task_id {
        "extract_posting" => Some(&crate::task::EXTRACT_POSTING),
        "import_resume" => Some(&crate::task::IMPORT_RESUME),
        "score_match" => Some(&crate::task::SCORE_MATCH),
        "write_cover_letter" => Some(&crate::task::WRITE_COVER_LETTER),
        "tailor_resume" => Some(&crate::task::TAILOR_RESUME),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
