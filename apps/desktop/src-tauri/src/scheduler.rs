//! Runs the scheduling pass from `ceevee_agent::scheduler` on a timer.
//!
//! The pass compares the wall clock on every tick, which is also how sleep and
//! wake are handled: the first tick after the computer wakes sees the missed
//! slot and fires it once.

use std::time::Duration;

use ceevee_agent::scheduler::{default_tasks, tick};
use ceevee_core::EventBus;
use ceevee_db::Db;
use chrono::Local;

const TICK: Duration = Duration::from_secs(30);

/// Spawn the scheduler on Tauri's async runtime.
pub fn spawn(db: Db, bus: EventBus) {
    tauri::async_runtime::spawn(async move {
        let tasks = default_tasks();
        let mut interval = tokio::time::interval(TICK);
        loop {
            interval.tick().await;
            let due = tick(&db, &bus, &tasks, Local::now()).await;
            if !due.is_empty() {
                tracing::info!("scheduled tasks due: {due:?}");
            }
        }
    });
}
