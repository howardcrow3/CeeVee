//! Logs and crash reports: a rolling log file, a panic hook that writes a report, and the
//! commands the Settings page uses to show the last crash and open the log folder.

use std::path::PathBuf;
use std::sync::OnceLock;

use serde::Serialize;
use specta::Type;
use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;

/// Kept alive for the life of the app so buffered log lines are flushed on exit.
static GUARD: OnceLock<WorkerGuard> = OnceLock::new();

fn log_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_log_dir().map_err(|e| e.to_string())
}

/// Start logging to `<log dir>/ceevee.log.<date>` (7 days kept) and install the panic hook.
/// Call once, first thing in setup.
pub fn setup(app: &tauri::App) {
    let Ok(dir) = log_dir(app.handle()) else {
        return;
    };
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("ceevee.log")
        .max_log_files(7)
        .build(&dir);
    if let Ok(appender) = appender {
        let (writer, guard) = tracing_appender::non_blocking(appender);
        let _ = GUARD.set(guard);
        let _ = tracing_subscriber::fmt()
            .with_writer(writer)
            .with_ansi(false)
            .with_max_level(tracing::Level::INFO)
            .try_init();
    }
    let hook_dir = dir.clone();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown error".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        tracing::error!("panic at {location}: {message}");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let os = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
        let text = ceevee_core::crash::format_report(&build_label(), &os, now, &message, &location);
        let _ = ceevee_core::crash::write_report(&hook_dir, now, &text);
        previous(info);
    }));
    tracing::info!("CeeVee {} started", build_label());
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Diagnostics {
    pub version: String,
    pub system: String,
    pub log_dir: String,
    /// What went wrong last time, when CeeVee closed unexpectedly and the person was not told yet.
    pub last_crash: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub fn diagnostics(app: tauri::AppHandle) -> Result<Diagnostics, String> {
    let dir = log_dir(&app)?;
    Ok(Diagnostics {
        version: build_label(),
        system: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        last_crash: ceevee_core::crash::latest_unseen(&dir).map(|c| c.summary),
        log_dir: dir.display().to_string(),
    })
}

/// The person has read the crash notice.
#[tauri::command]
#[specta::specta]
pub fn dismiss_crash(app: tauri::AppHandle) -> Result<(), String> {
    ceevee_core::crash::mark_all_seen(&log_dir(&app)?);
    Ok(())
}

/// Show the log folder in Explorer or Finder.
#[tauri::command]
#[specta::specta]
pub fn open_log_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = log_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// "0.1.0" for a tagged release, "0.1.0 build 57 (abc1234)" for a CI test build, so builds can be told apart.
pub fn build_label() -> String {
    match (
        option_env!("CEEVEE_BUILD_NUMBER"),
        option_env!("CEEVEE_BUILD_SHA"),
    ) {
        (Some(n), Some(sha)) if !n.is_empty() => {
            format!("{} build {n} ({sha})", env!("CARGO_PKG_VERSION"))
        }
        _ => env!("CARGO_PKG_VERSION").into(),
    }
}
