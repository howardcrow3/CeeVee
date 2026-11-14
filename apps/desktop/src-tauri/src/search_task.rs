//! Runs the daily job search when the scheduler says `search.daily` is due.

use ceevee_core::{Event, EventBus};
use ceevee_db::Db;
use ceevee_search::sources::{self, HttpFetcher};

/// Listen for the search task on Tauri's async runtime.
pub fn spawn(db: Db, bus: EventBus) {
    let mut rx = bus.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(Event::TaskDue { task }) if task == "search.daily" => {
                    match run_once(&db).await {
                        Ok(s) => tracing::info!(
                            "daily search: {} new, {} already known, {} problems",
                            s.new,
                            s.duplicates,
                            s.failures.len()
                        ),
                        Err(e) => tracing::warn!("daily search failed: {e}"),
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn run_once(db: &Db) -> ceevee_core::Result<sources::SearchSummary> {
    let boards = sources::load_boards(db).await?;
    if boards.is_empty() {
        return Ok(sources::SearchSummary::default());
    }
    let fetcher = HttpFetcher::new()?;
    sources::run(db, &fetcher, &boards, &sources::now_iso()).await
}
