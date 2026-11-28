//! The user's profile: contact details, experience, education, skills and saved answers, plus
//! importing a resume as a set of reviewable changes (TECHNICAL_PLAN sections 5 and 7).
//!
//! The profile is the single source of truth for everything CeeVee writes about the user. AI output
//! never writes to it directly: an import is turned into [`Change`]s, the user reviews them, and
//! only accepted changes are applied.

pub mod extract;
pub mod import;
pub mod repo;
pub mod resume_json;

pub use import::{plan_import, ResumeDraft};
pub use repo::{describe, Basics, Change, Entry, EntryKind, Fact};
