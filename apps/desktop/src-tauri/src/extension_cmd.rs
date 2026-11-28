//! The browser extension's side of the app: starts the local bridge and lets the person approve
//! or remove browsers. What the extension may do is decided in `ceevee-bridge`.

use std::sync::{Arc, Mutex};

use ceevee_bridge::{Backend, Bridge};
use ceevee_db::Db;
use ceevee_docs::render::Paper;
use serde::Serialize;
use specta::Type;
use tauri::Manager;

use crate::license_cmd::{self, Service};

const TOKENS: &str = "extension.tokens";

/// The running bridge, once it has started.
#[derive(Default)]
pub struct ExtensionState(Mutex<Option<Arc<Bridge>>>);

impl ExtensionState {
    fn bridge(&self) -> Option<Arc<Bridge>> {
        self.0.lock().ok().and_then(|g| g.clone())
    }
}

/// Start the bridge in the background. A failure to start is logged; the app works without it.
pub fn setup(app: &tauri::App, db: Db) {
    app.manage(ExtensionState::default());
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let dir = match handle.path().app_data_dir() {
            Ok(d) => d.join("apply").join("extension"),
            Err(e) => {
                tracing::warn!("extension bridge not started: {e}");
                return;
            }
        };
        let tokens: Vec<String> = db
            .get_setting(TOKENS)
            .await
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let (answers_db, doc_db, save_db) = (db.clone(), db.clone(), db.clone());
        let (answers_handle, doc_handle) = (handle.clone(), handle.clone());
        let (doc_dir, answers_dir) = (dir.clone(), dir);
        let backend = Backend {
            answers: Box::new(move || {
                let (db, handle, dir) = (
                    answers_db.clone(),
                    answers_handle.clone(),
                    answers_dir.clone(),
                );
                Box::pin(async move { answers(&db, &handle, &dir).await })
            }),
            document: Box::new(move |which| {
                let (db, handle, dir) = (doc_db.clone(), doc_handle.clone(), doc_dir.clone());
                Box::pin(async move {
                    if which != "resume" {
                        return Ok(None);
                    }
                    let a = answers(&db, &handle, &dir).await?;
                    let Some(path) = a.resume_pdf else {
                        return Ok(None);
                    };
                    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                    let name = std::path::Path::new(&path)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "resume.pdf".into());
                    Ok(Some((name, bytes)))
                })
            }),
            tokens_changed: Box::new(move |list| {
                let db = save_db.clone();
                tauri::async_runtime::spawn(async move {
                    let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
                    if let Err(e) = db.set_setting(TOKENS, &json).await {
                        tracing::warn!("could not save the connected browsers: {e}");
                    }
                });
            }),
        };
        match Bridge::start(backend, tokens).await {
            Ok(b) => {
                if let Some(state) = handle.try_state::<ExtensionState>() {
                    if let Ok(mut g) = state.0.lock() {
                        *g = Some(Arc::new(b));
                    }
                }
            }
            Err(e) => tracing::warn!("extension bridge not started: {e}"),
        }
    });
}

/// The person's answers, only while the trial or a license allows working.
async fn answers(
    db: &Db,
    handle: &tauri::AppHandle,
    dir: &std::path::Path,
) -> Result<ceevee_apply::engine::Answers, String> {
    if let Some(svc) = handle.try_state::<Arc<Service>>() {
        license_cmd::require_write(&svc).await?;
    }
    ceevee_agent::apply_prep::gather_general(db, dir, Paper::UsLetter)
        .await
        .map_err(|e| e.to_string())
}

/// What the Browser tab shows.
#[derive(Debug, Clone, Serialize, Type)]
pub struct ExtensionView {
    /// True once the local connection is up.
    pub running: bool,
    /// How many browsers are connected.
    pub connected: u32,
    /// A browser asking to connect, waiting for approval: its id and name. Empty when none.
    pub pending_id: String,
    pub pending_browser: String,
}

#[tauri::command]
#[specta::specta]
pub fn extension_status(state: tauri::State<'_, ExtensionState>) -> ExtensionView {
    match state.bridge() {
        None => ExtensionView {
            running: false,
            connected: 0,
            pending_id: String::new(),
            pending_browser: String::new(),
        },
        Some(b) => {
            let p = b.pending();
            ExtensionView {
                running: true,
                connected: u32::try_from(b.connected()).unwrap_or(u32::MAX),
                pending_id: p.as_ref().map(|p| p.id.clone()).unwrap_or_default(),
                pending_browser: p.map(|p| p.browser).unwrap_or_default(),
            }
        }
    }
}

/// Let the browser that is asking connect.
#[tauri::command]
#[specta::specta]
pub fn extension_approve(
    state: tauri::State<'_, ExtensionState>,
    id: String,
) -> Result<(), String> {
    let b = state.bridge().ok_or("The connection is not running.")?;
    if b.approve(&id) {
        Ok(())
    } else {
        Err("That request has expired. Ask again from the browser.".into())
    }
}

/// Turn the browser that is asking away.
#[tauri::command]
#[specta::specta]
pub fn extension_deny(state: tauri::State<'_, ExtensionState>, id: String) {
    if let Some(b) = state.bridge() {
        b.deny(&id);
    }
}

/// Disconnect every browser; each must be approved again.
#[tauri::command]
#[specta::specta]
pub fn extension_disconnect_all(state: tauri::State<'_, ExtensionState>) {
    if let Some(b) = state.bridge() {
        b.disconnect_all();
    }
}
