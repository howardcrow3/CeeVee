//! Run a task's eval cases against a local model and print the scores.
//!
//! ```text
//! cargo run --release -p ceevee-ai --features llama --bin ceevee-eval -- <model.gguf> [task]
//! ```
//!
//! Environment: `CEEVEE_EVAL_BUDGET_SECS` stops starting new cases after that many seconds, and
//! `CEEVEE_EVAL_MIN_SCORE` (0 to 1) makes the exit code 1 when the mean score is below it.
//! `CEEVEE_EVAL_JSON` is a file to write the report to as JSON, labelled with `CEEVEE_EVAL_MODEL`.

#[cfg(feature = "llama")]
fn main() {
    use std::time::Duration;

    use ceevee_ai::eval::{builtin_cases, builtin_task, parse_cases, run_eval_within};
    use ceevee_ai::llama::LlamaCppBackend;
    use ceevee_ai::ModelWorker;

    let mut args = std::env::args().skip(1);
    let Some(model) = args.next() else {
        eprintln!("usage: ceevee-eval <model.gguf> [task]");
        std::process::exit(2);
    };
    let task = args.next().unwrap_or_else(|| "extract_posting".into());
    if task == "bench" {
        bench(&model);
        return;
    }
    let Some(spec) = builtin_task(&task) else {
        eprintln!("unknown task {task}");
        std::process::exit(2);
    };
    let cases = parse_cases(builtin_cases(&task).expect("built-in cases")).expect("cases parse");
    let env_f64 = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<f64>().ok());
    let budget = env_f64("CEEVEE_EVAL_BUDGET_SECS").map(Duration::from_secs_f64);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let worker = ModelWorker::spawn(LlamaCppBackend::new(model), Duration::from_secs(600));
    let report = rt.block_on(run_eval_within(&worker, spec, &cases, budget));
    print!("{}", report.summary());
    if let Ok(path) = std::env::var("CEEVEE_EVAL_JSON") {
        let label = std::env::var("CEEVEE_EVAL_MODEL").unwrap_or_else(|_| "unknown".into());
        let json = serde_json::to_string_pretty(&report.to_json(&label)).expect("report json");
        std::fs::write(&path, json).expect("write eval json");
    }
    if let Some(min) = env_f64("CEEVEE_EVAL_MIN_SCORE") {
        if report.mean_score() < min {
            eprintln!(
                "mean score {:.2} is below the floor {min:.2}",
                report.mean_score()
            );
            std::process::exit(1);
        }
    }
}

/// Speed check: tokens per second with no grammar and with each task's grammar. Used to find out
/// why eval cases are slow on a runner (prompt processing, plain decoding, or grammar sampling).
#[cfg(feature = "llama")]
fn bench(model: &str) {
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;

    use ceevee_ai::eval::builtin_task;
    use ceevee_ai::llama::LlamaCppBackend;
    use ceevee_ai::{GenerateRequest, LlmBackend};

    let started = Instant::now();
    let mut backend = LlamaCppBackend::new(model);
    backend.load().expect("load model");
    println!("load: {:.1}s", started.elapsed().as_secs_f64());
    let cancel = AtomicBool::new(false);
    let cases: Vec<(&str, Option<String>)> = vec![
        ("no grammar", None),
        ("score_match grammar", builtin_task("score_match").and_then(|s| s.grammar().ok())),
        ("extract_posting grammar", builtin_task("extract_posting").and_then(|s| s.grammar().ok())),
    ];
    for (name, grammar) in cases {
        let mut req = GenerateRequest::new(
            "Posting: Data analyst at Acme Corp, remote, $90,000 to $110,000. Answer with JSON describing it.",
        );
        req.grammar = grammar;
        req.max_tokens = 48;
        let begun = Instant::now();
        let mut first = None;
        let mut pieces = 0u32;
        let out = backend.generate(&req, &cancel, &mut |_| {
            pieces += 1;
            first.get_or_insert_with(|| begun.elapsed());
        });
        let total = begun.elapsed().as_secs_f64();
        let first = first.map_or(0.0, |d| d.as_secs_f64());
        let rest = (total - first).max(0.0001);
        println!(
            "{name}: {pieces} pieces, first piece after {first:.2}s, then {:.2} pieces/s, total {total:.1}s, ok={}",
            f64::from(pieces.saturating_sub(1)) / rest,
            out.is_ok()
        );
    }
}

#[cfg(not(feature = "llama"))]
fn main() {
    eprintln!("build with --features llama to run evals against a model");
    std::process::exit(2);
}
