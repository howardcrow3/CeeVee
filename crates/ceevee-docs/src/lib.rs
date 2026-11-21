//! Documents: base resumes, variants, tailored resumes, cover letters and emails, each with a
//! version history (TECHNICAL_PLAN section 7). Rendering (Typst, DOCX) comes later.

pub mod chat;
pub mod draft;
pub mod export;
pub mod fact_guard;
pub mod render;
pub mod store;

pub use fact_guard::{check as fact_check, check_value as fact_check_value, Flag, FlagKind};
pub use store::{DocKind, Document, Version};
