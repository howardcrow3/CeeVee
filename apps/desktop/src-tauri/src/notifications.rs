//! Shows desktop notifications for the types defined in
//! `ceevee_agent::notifications` (TECHNICAL_PLAN 3.1).

use ceevee_agent::notifications::{is_enabled, Kind};
use ceevee_db::Db;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Show a notification if its type is enabled. Failures are logged, not raised.
pub async fn notify(app: &AppHandle, db: &Db, kind: Kind, title: &str, body: &str) {
    if !is_enabled(db, kind).await {
        return;
    }
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!("notification failed: {e}");
    }
}
