//! Domain types, errors and the event bus shared by every CeeVee crate.
//!
//! Feature crates never call each other; they publish and subscribe on the
//! [`EventBus`] (docs/TECHNICAL_PLAN.md, section 2).

mod bus;
mod error;

pub use bus::{Event, EventBus};
pub use error::{Error, Result};

/// Basic facts about the running app, returned to the UI at start-up.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub schema_version: u32,
}
