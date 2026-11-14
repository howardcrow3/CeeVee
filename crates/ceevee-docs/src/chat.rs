//! Chat editing: the user asks for a change in plain words, the model returns the whole new
//! document, and code decides whether it may be saved. This module holds the pure parts (the
//! prompt, cleaning the reply, the fact check) so they are tested without a model.

use crate::fact_guard::{self, Flag};
use crate::store::DocKind;

/// The prompt for one edit. The model must answer with the full document and nothing else.
pub fn edit_prompt(kind: DocKind, content: &str, instruction: &str) -> String {
    let (what, format) = match kind {
        DocKind::BaseResume | DocKind::ResumeVariant | DocKind::TailoredResume => (
            "resume",
            "valid JSON in exactly the same shape as the original",
        ),
        DocKind::CoverLetter => ("cover letter", "plain text"),
        DocKind::Email => ("email", "plain text"),
    };
    format!(
        "You edit a {what} for the person who owns it. Apply the change they ask for and return the \
         complete updated {what} as {format}. Do not add facts, numbers, employers or skills that are \
         not already in the {what} or in the request. Return only the {what}, with no commentary and \
         no code fences.\n\n<document>\n{content}\n</document>\n\n<request>\n{instruction}\n</request>"
    )
}

/// Strip what models wrap around an answer: a thinking block and a code fence.
pub fn clean_reply(raw: &str) -> String {
    let mut s = raw.trim();
    if let Some(end) = s.rfind("</think>") {
        s = s[end + "</think>".len()..].trim();
    }
    if let Some(rest) = s.strip_prefix("```") {
        let rest = rest.split_once('\n').map_or("", |(_, body)| body);
        s = rest.trim_end().strip_suffix("```").unwrap_or(rest).trim();
    }
    s.to_string()
}

/// What the fact guard thinks of an edit: names or numbers that were in neither the old document
/// nor the request.
pub fn check_edit(old: &str, instruction: &str, new: &str) -> Vec<Flag> {
    fact_guard::check(new, &[old, instruction])
}

#[cfg(test)]
mod tests;
