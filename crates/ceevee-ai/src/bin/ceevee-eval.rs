//! Run a task's eval cases against a local model and print the scores.
//!
//! ```text
//! cargo run --release -p ceevee-ai --features llama --bin ceevee-eval -- <model.gguf> [task]
//! ```

#[cfg(feature = "llama")]
fn main() {
    use std::time::Duration;

    use ceevee_ai::eval::{builtin_cases, parse_cases, run_eval};
    use ceevee_ai::llama::LlamaCppBackend;
    use ceevee_ai::task::EXTRACT_POSTING;
    use ceevee_ai::ModelWorker;

    let mut args = std::env::args().skip(1);
    let Some(model) = args.next() else {
        eprintln!("usage: ceevee-eval <model.gguf> [task]");
        std::process::exit(2);
    };
    let task = args.next().unwrap_or_else(|| "extract_posting".into());
    let spec = match task.as_str() {
        "extract_posting" => &EXTRACT_POSTING,
        other => {
            eprintln!("unknown task {other}");
            std::process::exit(2);
        }
    };
    let cases = parse_cases(builtin_cases(&task).expect("built-in cases")).expect("cases parse");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let worker = ModelWorker::spawn(LlamaCppBackend::new(model), Duration::from_secs(600));
    let report = rt.block_on(run_eval(&worker, spec, &cases));
    print!("{}", report.summary());
}

#[cfg(not(feature = "llama"))]
fn main() {
    eprintln!("build with --features llama to run evals against a model");
    std::process::exit(2);
}
