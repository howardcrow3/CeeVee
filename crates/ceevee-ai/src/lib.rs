//! Local AI: model catalog, hardware check and resumable model downloads.
//! The llama.cpp runtime, tasks and grammars arrive in later PRs
//! (docs/TECHNICAL_PLAN.md, section 5).

pub mod catalog;
pub mod download;
pub mod hardware;

pub use catalog::{Catalog, ModelEntry, Role};
