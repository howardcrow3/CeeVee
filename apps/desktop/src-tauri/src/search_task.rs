//! Runs the daily job search when the scheduler says `search.daily` is due.

use ceevee_agent::notifications::Kind;
use ceevee_core::{Event, EventBus};
use ceevee_db::Db;
use ceevee_search::alerts;
use ceevee_search::sources::{self, HttpFetcher};
use tauri::{AppHandle, Manager};

use crate::notifications::notify;

/// Listen for the search task on Tauri's async runtime.
pub fn spawn(app: AppHandle, db: Db, bus: EventBus) {
    let mut rx = bus.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(Event::TaskDue { task }) if task == "search.daily" => {
                    if let Some(lic) =
                        app.try_state::<std::sync::Arc<crate::license_cmd::Service>>()
                    {
                        if crate::license_cmd::require_write(&lic).await.is_err() {
                            tracing::info!("daily search skipped: trial ended, read-only");
                            continue;
                        }
                    }
                    let started = sources::now_iso();
                    match run_once(&db).await {
                        Ok(s) => {
                            tracing::info!(
                                "daily search: {} new, {} already known, {} problems",
                                s.new,
                                s.duplicates,
                                s.failures.len()
                            );
                            alert_new_matches(&app, &db, &started).await;
                            // Lets the tray menu refresh its count right away.
                            bus.publish(Event::SettingChanged {
                                key: "search.last_run".into(),
                            });
                        }
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
    let fetcher = HttpFetcher::new()?;
    ceevee_search::starter::run_all(db, &fetcher, &sources::now_iso()).await
}

/// Tell the user about new postings that score well, if "new matches" notifications are on.
async fn alert_new_matches(app: &AppHandle, db: &Db, since: &str) {
    let cutoff = alerts::min_score(db).await;
    match alerts::new_matches(db, since, cutoff).await {
        Ok(found) => {
            if let Some((title, body)) = alerts::message(&found) {
                notify(app, db, Kind::NewMatches, &title, &body).await;
            }
        }
        Err(e) => tracing::warn!("could not check for new matches: {e}"),
    }
}
