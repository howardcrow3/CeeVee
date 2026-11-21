//! Turning a stored document into a PDF file: which renderer to use and a safe file name.

use ceevee_core::{Error, Result};

use crate::render::{letter_pdf, resume_pdf, Paper};
use crate::store::DocKind;

/// A file name made from a document title: letters, digits, spaces, dashes and underscores only,
/// so it is safe on Windows and macOS. Never empty.
pub fn file_stem(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') {
                c
            } else {
                ' '
            }
        })
        .collect();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let joined = words.join(" ");
    let short: String = joined.chars().take(80).collect();
    if short.is_empty() {
        "document".to_string()
    } else {
        short
    }
}

/// Render a stored document to PDF bytes. Resumes are JSON; letters and emails are plain text.
pub fn pdf_for(kind: DocKind, content: &str, paper: Paper) -> Result<Vec<u8>> {
    match kind {
        DocKind::BaseResume | DocKind::ResumeVariant | DocKind::TailoredResume => {
            let v: serde_json::Value = serde_json::from_str(content)
                .map_err(|e| Error::Invalid(format!("The resume is not valid JSON: {e}")))?;
            resume_pdf(&v, paper)
        }
        DocKind::CoverLetter | DocKind::Email => letter_pdf(content, paper),
    }
}

#[cfg(test)]
mod tests;
