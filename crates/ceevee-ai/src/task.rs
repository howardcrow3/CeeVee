//! The task framework: a versioned prompt, a JSON Schema for the answer, and a runner that
//! renders the prompt, constrains the model with a grammar, parses and validates the output, and
//! retries once with the error when the model still gets it wrong (TECHNICAL_PLAN section 5).
//!
//! Prompts live in `prompts/` and are compiled in. Change a prompt's text, bump its `version`;
//! stored results record the version they came from.

use ceevee_core::{Error, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::backend::GenerateRequest;
use crate::grammar::json_schema_to_gbnf;
use crate::validate::validate;
use crate::worker::{ModelWorker, Priority};

/// How many times the model is asked before the task fails.
const MAX_ATTEMPTS: u32 = 2;

pub struct TaskSpec {
    pub id: &'static str,
    pub version: u32,
    /// Prompt text with `{{name}}` placeholders.
    pub prompt: &'static str,
    /// JSON Schema for the answer, as JSON text.
    pub schema: &'static str,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskOutput<T> {
    pub value: T,
    /// The model's raw text for the accepted attempt.
    pub raw: String,
    pub attempts: u32,
    pub task_id: &'static str,
    pub version: u32,
}

/// Fill `{{name}}` placeholders. A missing variable or a leftover placeholder is an error, so a
/// typo never reaches the model. Values are inserted as is and are not scanned again.
pub fn render(template: &str, vars: &[(&str, &str)]) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| Error::Invalid("unclosed {{ in prompt template".into()))?;
        let name = after[..end].trim();
        let value = vars
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| *v)
            .ok_or_else(|| Error::Invalid(format!("prompt variable \"{name}\" not provided")))?;
        out.push_str(value);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

impl TaskSpec {
    pub fn schema_value(&self) -> Result<Value> {
        serde_json::from_str(self.schema)
            .map_err(|e| Error::Invalid(format!("task {} schema is not JSON: {e}", self.id)))
    }

    pub fn grammar(&self) -> Result<String> {
        json_schema_to_gbnf(&self.schema_value()?)
    }
}

/// Parse and validate model text against the task's schema, then convert to `T`.
pub fn parse_output<T: DeserializeOwned>(
    schema: &Value,
    raw: &str,
) -> std::result::Result<T, String> {
    let value: Value =
        serde_json::from_str(raw.trim()).map_err(|e| format!("not valid JSON: {e}"))?;
    validate(schema, &value)?;
    serde_json::from_value(value).map_err(|e| format!("does not fit the expected type: {e}"))
}

/// Run a task on the model worker.
pub async fn run<T: DeserializeOwned>(
    worker: &ModelWorker,
    spec: &TaskSpec,
    vars: &[(&str, &str)],
    priority: Priority,
) -> Result<TaskOutput<T>> {
    let schema = spec.schema_value()?;
    let grammar = spec.grammar()?;
    let base = render(spec.prompt, vars)?;
    let mut prompt = base.clone();
    let mut last_problem = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        let mut req = GenerateRequest::new(prompt.clone());
        req.grammar = Some(grammar.clone());
        req.max_tokens = spec.max_tokens;
        req.temperature = spec.temperature;
        let job = worker.submit(priority, req, None);
        let raw = job
            .result
            .await
            .map_err(|_| Error::Other("model worker stopped".into()))??;
        match parse_output::<T>(&schema, &raw) {
            Ok(value) => {
                return Ok(TaskOutput {
                    value,
                    raw,
                    attempts: attempt,
                    task_id: spec.id,
                    version: spec.version,
                })
            }
            Err(problem) => {
                prompt = format!(
                    "{base}\n\nYour previous answer was rejected: {problem}\nAnswer again with JSON that fixes this."
                );
                last_problem = problem;
            }
        }
    }
    Err(Error::Invalid(format!(
        "task {} failed after {MAX_ATTEMPTS} attempts: {last_problem}",
        spec.id
    )))
}

/// Extract the facts a posting states (title, company, pay, skills).
pub const EXTRACT_POSTING: TaskSpec = TaskSpec {
    id: "extract_posting",
    version: 1,
    prompt: include_str!("../prompts/extract_posting.md"),
    schema: include_str!("../prompts/extract_posting.schema.json"),
    max_tokens: 600,
    temperature: 0.0,
};

/// Copy the facts of a resume's text into a structured record. Long resumes are read in one
/// piece, so the caller must keep the text within the model's window.
pub const IMPORT_RESUME: TaskSpec = TaskSpec {
    id: "import_resume",
    version: 1,
    prompt: include_str!("../prompts/import_resume.md"),
    schema: include_str!("../prompts/import_resume.schema.json"),
    max_tokens: 1800,
    temperature: 0.0,
};

#[cfg(test)]
mod tests;
