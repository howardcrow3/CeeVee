//! In-app updates: check the signed manifest and install on request. The
//! public key and endpoint live in `tauri.conf.json`; update files are signed
//! at release time with the private updater key.

use serde::Serialize;
use specta::Type;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize, Type)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

/// Ask the update server whether a newer version exists.
#[tauri::command]
#[specta::specta]
pub async fn check_for_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let checked = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(e) => {
            tracing::warn!("update check could not start: {e}");
            return Ok(None);
        }
    };
    // Until a release is published the feed does not exist, and the check fails.
    // Treat any failed check as "nothing newer" so the user sees "up to date"
    // rather than a raw error; the cause goes to the log.
    let update = match checked {
        Ok(update) => update,
        Err(e) => {
            tracing::warn!("update check failed, reporting up to date: {e}");
            None
        }
    };
    Ok(update.map(|u| UpdateInfo {
        version: u.version.clone(),
        notes: u.body.clone(),
    }))
}

/// Download, verify (signature) and install the newest version, then restart.
#[tauri::command]
#[specta::specta]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let Some(update) = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}
