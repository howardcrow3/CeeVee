//! Drafting pipelines: run a drafting task, then the fact guard, and only save what passes
//! (TECHNICAL_PLAN section 5). A draft with flags is returned to the caller, who shows them to the
//! user; the user either edits the draft or accepts specific flagged items, and only then is it saved.

use ceevee_ai::task::{run, TAILOR_RESUME, WRITE_COVER_LETTER};
use ceevee_ai::{ModelWorker, Priority};
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde::Deserialize;
use serde_json::Value;

use crate::fact_guard::{check, check_value, check_value_split, Flag};
use crate::store;

#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    /// Letter Markdown, or resume JSON text.
    pub content: String,
    pub flags: Vec<Flag>,
    /// Task and prompt version, such as `write_cover_letter@1`.
    pub created_with: String,
}

impl Draft {
    /// Flags the user has not accepted. `accepted` holds the flagged texts the user chose to keep.
    pub fn unresolved(&self, accepted: &[String]) -> Vec<&Flag> {
        self.flags
            .iter()
            .filter(|f| !accepted.iter().any(|a| a.eq_ignore_ascii_case(&f.text)))
            .collect()
    }
}

#[derive(Deserialize)]
struct Letter {
    letter: String,
}

/// Write a cover letter and check it against the profile and posting.
pub async fn draft_cover_letter(
    worker: &ModelWorker,
    profile_text: &str,
    posting_text: &str,
    instructions: &str,
) -> Result<Draft> {
    let extra = if instructions.trim().is_empty() {
        String::new()
    } else {
        format!("- Also: {}", instructions.trim())
    };
    let vars = [
        ("profile", profile_text),
        ("posting", posting_text),
        ("instructions", extra.as_str()),
    ];
    let out = run::<Letter>(worker, &WRITE_COVER_LETTER, &vars, Priority::Interactive).await?;
    let flags = check(&out.value.letter, &[profile_text, posting_text]);
    Ok(Draft {
        content: out.value.letter,
        flags,
        created_with: format!("{}@{}", out.task_id, out.version),
    })
}

/// Tailor a resume (JSON in the `import_resume` shape) to a posting and check that it only uses
/// what the base resume says. Numbers must come from the resume itself; names may also come from
/// the posting, because tailoring may use the posting's own term for the same thing.
pub async fn draft_tailored_resume(
    worker: &ModelWorker,
    base_resume: &Value,
    posting_text: &str,
) -> Result<Draft> {
    let base_text = base_resume.to_string();
    let vars = [("resume", base_text.as_str()), ("posting", posting_text)];
    let out = run::<Value>(worker, &TAILOR_RESUME, &vars, Priority::Interactive).await?;
    let own = crate::fact_guard::text_of(base_resume);
    let flags = check_value_split(&out.value, &[own.as_str(), posting_text], &[own.as_str()]);
    Ok(Draft {
        content: out.value.to_string(),
        flags,
        created_with: format!("{}@{}", out.task_id, out.version),
    })
}

/// Check text the user typed or edited, with the same rules. Used when the user changes a draft
/// by hand after flags were shown.
pub fn recheck_letter(content: &str, profile_text: &str, posting_text: &str) -> Vec<Flag> {
    check(content, &[profile_text, posting_text])
}

pub fn recheck_resume(content: &Value, profile_text: &str) -> Vec<Flag> {
    check_value(content, &[profile_text])
}

/// Save a draft as a new version of `doc_id`. Refuses while flags remain that the user has not
/// accepted. Returns the new version id.
pub async fn save_draft(
    db: &Db,
    doc_id: i64,
    draft: &Draft,
    accepted: &[String],
    cause: &str,
) -> Result<i64> {
    let open = draft.unresolved(accepted);
    if !open.is_empty() {
        let list: Vec<String> = open.iter().map(|f| format!("\"{}\"", f.text)).collect();
        return Err(Error::Invalid(format!(
            "the draft says things the profile and posting do not: {}",
            list.join(", ")
        )));
    }
    store::add_version(db, doc_id, &draft.content, cause, Some(&draft.created_with)).await
}

#[cfg(test)]
mod tests;
