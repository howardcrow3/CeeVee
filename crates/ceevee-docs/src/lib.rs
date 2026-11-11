//! Documents: base resumes, variants, tailored resumes, cover letters and emails, each with a
//! version history (TECHNICAL_PLAN section 7). Rendering (Typst, DOCX) comes later.

pub mod store;

pub use store::{DocKind, Document, Version};
