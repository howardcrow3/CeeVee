//! Job search: application tracking, companies, postings with de-duplication, and rule-based
//! scoring against the user's job categories. Sources and the scheduler come in later milestones
//! (see docs/TECHNICAL_PLAN.md).

pub mod categories;
pub mod companies;
pub mod postings;
pub mod tracking;

fn db_err(e: sqlx::Error) -> ceevee_core::Error {
    ceevee_core::Error::Database(e.to_string())
}

/// Lowercase words only, so "Sr. Engineer (Remote)" and "sr engineer remote" compare equal.
pub(crate) fn words(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
