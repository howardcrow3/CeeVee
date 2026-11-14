//! The local model: what the machine can run, and a download you can watch and cancel.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ceevee_ai::catalog::{Catalog, Role};
use serde::Serialize;
use specta::Type;
use tauri::Manager;

/// Shared between the download task and the commands that ask how it is going.
#[derive(Default)]
pub struct DownloadState {
    downloaded: AtomicU64,
    total: AtomicU64,
    running: AtomicBool,
    cancel: AtomicBool,
    error: Mutex<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ModelStatus {
    pub ram_gb: u32,
    pub id: String,
    pub name: String,
    pub size_mb: u32,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct DownloadProgress {
    pub running: bool,
    pub downloaded_mb: u32,
    pub total_mb: u32,
    pub error: String,
}

fn models_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("models"))
}

fn mb(bytes: u64) -> u32 {
    (bytes / 1_000_000) as u32
}

/// The model suited to this machine, and whether it is already on disk.
#[tauri::command]
#[specta::specta]
pub async fn model_status(app: tauri::AppHandle) -> Result<ModelStatus, String> {
    let ram = ceevee_ai::hardware::total_ram_gb();
    let catalog = Catalog::builtin();
    let entry = catalog
        .recommend(Role::General, ram)
        .ok_or("This computer does not have enough memory for the built-in models.")?;
    let installed = models_dir(&app)?.join(&entry.file).exists();
    Ok(ModelStatus {
        ram_gb: ram as u32,
        id: entry.id.clone(),
        name: entry.name.clone(),
        size_mb: mb(entry.size_bytes),
        installed,
    })
}

/// Start downloading the model in the background. Progress comes from `model_progress`.
#[tauri::command]
#[specta::specta]
pub async fn start_model_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<DownloadState>>,
    id: String,
) -> Result<(), String> {
    let entry = Catalog::builtin()
        .get(&id)
        .cloned()
        .ok_or("Unknown model.")?;
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("A download is already running.".into());
    }
    state.cancel.store(false, Ordering::SeqCst);
    state.downloaded.store(0, Ordering::SeqCst);
    state.total.store(entry.size_bytes, Ordering::SeqCst);
    if let Ok(mut e) = state.error.lock() {
        e.clear();
    }
    let dir = models_dir(&app)?;
    let st = Arc::clone(&state);
    tauri::async_runtime::spawn(async move {
        let result = ceevee_ai::download::download(&entry, &dir, &st.cancel, |p| {
            st.downloaded.store(p.downloaded, Ordering::Relaxed);
            st.total.store(p.total, Ordering::Relaxed);
        })
        .await;
        if let Err(e) = result {
            if !st.cancel.load(Ordering::SeqCst) {
                if let Ok(mut m) = st.error.lock() {
                    *m = e.to_string();
                }
            }
        }
        st.running.store(false, Ordering::SeqCst);
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn model_progress(state: tauri::State<'_, Arc<DownloadState>>) -> DownloadProgress {
    DownloadProgress {
        running: state.running.load(Ordering::SeqCst),
        downloaded_mb: mb(state.downloaded.load(Ordering::Relaxed)),
        total_mb: mb(state.total.load(Ordering::Relaxed)),
        error: state.error.lock().map(|e| e.clone()).unwrap_or_default(),
    }
}

/// Stop the download. What was fetched is kept, so starting again resumes.
#[tauri::command]
#[specta::specta]
pub fn cancel_model_download(state: tauri::State<'_, Arc<DownloadState>>) {
    state.cancel.store(true, Ordering::SeqCst);
}

/// The running engine, started the first time the AI is asked for something.
#[derive(Default)]
pub struct Engine {
    worker: Mutex<Option<ceevee_ai::ModelWorker>>,
}

#[cfg(feature = "llama")]
fn start_worker(path: PathBuf) -> Result<ceevee_ai::ModelWorker, String> {
    let backend = ceevee_ai::llama::LlamaCppBackend::new(path);
    Ok(ceevee_ai::ModelWorker::spawn(
        backend,
        std::time::Duration::from_secs(300),
    ))
}

#[cfg(not(feature = "llama"))]
fn start_worker(_path: PathBuf) -> Result<ceevee_ai::ModelWorker, String> {
    Err("This build of CeeVee does not include the local AI engine.".into())
}

/// Ask the local model one question and wait for the whole answer.
#[tauri::command]
#[specta::specta]
pub async fn ask_model(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<Engine>>,
    prompt: String,
) -> Result<String, String> {
    generate(&app, &engine, prompt).await
}

/// Run one prompt on the local model, starting the engine if needed.
pub async fn generate(
    app: &tauri::AppHandle,
    engine: &Engine,
    prompt: String,
) -> Result<String, String> {
    let ram = ceevee_ai::hardware::total_ram_gb();
    let catalog = Catalog::builtin();
    let entry = catalog
        .recommend(Role::General, ram)
        .ok_or("This computer does not have enough memory for the built-in models.")?;
    let path = models_dir(app)?.join(&entry.file);
    if !path.exists() {
        return Err("Download the model in Settings first.".into());
    }
    let worker = {
        let mut slot = engine.worker.lock().map_err(|e| e.to_string())?;
        if slot.is_none() {
            *slot = Some(start_worker(path)?);
        }
        slot.clone().ok_or("The model did not start.")?
    };
    let job = worker.submit(
        ceevee_ai::Priority::Interactive,
        ceevee_ai::GenerateRequest::new(prompt),
        None,
    );
    job.result
        .await
        .map_err(|_| "The model stopped before it answered.".to_string())?
        .map_err(|e| e.to_string())
}
