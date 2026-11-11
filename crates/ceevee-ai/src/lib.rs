//! Local AI: model catalog, hardware check and resumable model downloads.
//! The llama.cpp runtime, tasks and grammars arrive in later PRs
//! (docs/TECHNICAL_PLAN.md, section 5).

pub mod backend;
pub mod catalog;
pub mod download;
pub mod grammar;
pub mod hardware;
pub mod worker;

pub use backend::{GenerateRequest, LlmBackend};
pub use catalog::{Catalog, ModelEntry, Role};
pub use worker::{CancelHandle, Job, ModelWorker, Priority};
