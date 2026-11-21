//! Analytics commands for the UI. All rules live in `ceevee-analytics`.
//!
//! Measurement ID and API secret are baked in at build time (`CEEVEE_GA4_MEASUREMENT_ID`,
//! `CEEVEE_GA4_API_SECRET`). Builds without them, including every dev build, send nothing.

use std::sync::Arc;

use ceevee_analytics::{Analytics, Consent, Context, Event, Target};
use serde::Serialize;
use specta::Type;
use tauri::Manager;

pub type Handle = Arc<Analytics>;

fn target() -> Option<Target> {
    match (
        option_env!("CEEVEE_GA4_MEASUREMENT_ID"),
        option_env!("CEEVEE_GA4_API_SECRET"),
    ) {
        (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => {
            Some(Target::new(id, secret))
        }
        _ => None,
    }
}

/// Build the service, and in the background count the app start (and, once, an unexpected
/// close last time). Both are skipped unless the person has agreed.
pub fn setup(app: &tauri::App, db: ceevee_db::Db) {
    let ctx = Context {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
    };
    let a: Handle = Arc::new(Analytics::new(db.clone(), target(), ctx));
    app.manage(a.clone());
    let log_dir = app.path().app_log_dir().ok();
    tauri::async_runtime::spawn(async move {
        a.track(Event::AppStart).await;
        if let Some(c) = log_dir.and_then(|d| ceevee_core::crash::latest_unseen(&d)) {
            let key = c.path.display().to_string();
            if db
                .get_setting("analytics_last_crash")
                .await
                .ok()
                .flatten()
                .as_deref()
                != Some(&key)
            {
                let _ = db.set_setting("analytics_last_crash", &key).await;
                a.track(Event::Crash).await;
            }
        }
    });
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct AnalyticsView {
    /// `unset` (not asked yet), `granted` or `denied`.
    pub consent: String,
    /// False in builds without Google credentials: the choice is saved but nothing is sent.
    pub available: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn analytics_status(a: tauri::State<'_, Handle>) -> Result<AnalyticsView, String> {
    let consent = match a.consent().await {
        Consent::Unset => "unset",
        Consent::Granted => "granted",
        Consent::Denied => "denied",
    };
    Ok(AnalyticsView {
        consent: consent.into(),
        available: target().is_some(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn set_analytics_consent(a: tauri::State<'_, Handle>, share: bool) -> Result<(), String> {
    let c = if share {
        Consent::Granted
    } else {
        Consent::Denied
    };
    a.set_consent(c).await.map_err(|e| e.to_string())?;
    if share {
        a.track(Event::AppStart).await;
    }
    Ok(())
}
