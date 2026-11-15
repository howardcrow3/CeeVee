//! `ceevee://activate?key=CEEV-…` links, from the purchase email and the "My license" page.
//! The operating system starts CeeVee (or hands the link to the running copy) and we activate.

use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::license_cmd::Service;

/// Sent to the UI after a link was handled, so the license panel can refresh and say what happened.
pub const EVENT: &str = "license-link";

#[derive(Debug, Clone, Serialize)]
pub struct LinkResult {
    pub ok: bool,
    pub message: String,
}

async fn handle(app: AppHandle, key: String) {
    crate::show_main_window(&app);
    let result = match app.try_state::<Arc<Service>>() {
        Some(svc) => match svc.activate(&key, Utc::now()).await {
            Ok(_) => LinkResult {
                ok: true,
                message: "Your license is active. Thank you!".into(),
            },
            Err(e) => LinkResult {
                ok: false,
                message: e.to_string(),
            },
        },
        None => LinkResult {
            ok: false,
            message: "CeeVee is still starting. Open the link again in a moment.".into(),
        },
    };
    let _ = app.emit(EVENT, result);
}

fn dispatch(app: &AppHandle, urls: Vec<String>) {
    // A link that is not an activation link still brings the window forward but changes nothing.
    match ceevee_license::key::key_from_links(urls) {
        Some(key) => {
            let app = app.clone();
            tauri::async_runtime::spawn(handle(app, key));
        }
        None => crate::show_main_window(app),
    }
}

pub fn setup(app: &tauri::App) {
    let handle = app.handle().clone();
    // Links that arrive while CeeVee runs (and, on Windows, ones passed to a second launch).
    app.deep_link().on_open_url({
        let handle = handle.clone();
        move |event| {
            dispatch(
                &handle,
                event.urls().iter().map(|u| u.to_string()).collect(),
            )
        }
    });
    // The link that started CeeVee.
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        dispatch(&handle, urls.iter().map(|u| u.to_string()).collect());
    }
    // Windows and Linux installs register the scheme; a development run has no installer, so do it here.
    #[cfg(any(windows, target_os = "linux"))]
    if cfg!(debug_assertions) {
        let _ = app.deep_link().register_all();
    }
}
