//! CeeVee desktop shell: window, tray, single instance and the typed command
//! surface the UI calls. Domain logic lives in the `ceevee-*` crates.

mod commands;
#[allow(dead_code)] // called by feature crates as they land
mod notifications;
mod scheduler;
mod search_task;
mod tray;
mod updater;

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

#[tauri::command]
#[specta::specta]
async fn app_info(db: tauri::State<'_, ceevee_db::Db>) -> Result<ceevee_core::AppInfo, String> {
    Ok(ceevee_core::AppInfo {
        name: "CeeVee".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        schema_version: db.schema_version().await.map_err(|e| e.to_string())? as u32,
    })
}

/// Typed command/event surface shared with the UI. `export_bindings` writes the
/// TypeScript side; a test keeps the committed file in sync.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            app_info,
            updater::check_for_update,
            updater::install_update,
            commands::import_linkedin_connections,
            commands::export_resume_pdf,
            commands::list_jobs,
            commands::list_boards,
            commands::save_boards,
            commands::run_search,
            commands::list_proposals,
            commands::skip_proposal,
            commands::approve_proposal,
            commands::pending_approval_count
        ])
        .typ::<ceevee_core::Event>()
}

pub const BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ui/src/lib/bindings.ts");

/// Write `ui/src/lib/bindings.ts`. Run from `apps/desktop/src-tauri`:
/// `cargo run -- --export-bindings`.
pub fn export_bindings() -> Result<(), Box<dyn std::error::Error>> {
    specta_builder().export(specta_typescript::Typescript::default(), BINDINGS_PATH)?;
    Ok(())
}

pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Launch at login is on by default; the first run switches it on and records
/// that it did, so a later "off" in Settings sticks.
fn launch_at_login_default(app: &tauri::App) {
    use tauri_plugin_autostart::ManagerExt;
    let db = app.state::<ceevee_db::Db>().inner().clone();
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        if db
            .get_setting("launch_at_login.initialized")
            .await
            .ok()
            .flatten()
            .is_some()
        {
            return;
        }
        if let Err(e) = handle.autolaunch().enable() {
            tracing::warn!("could not enable launch at login: {e}");
        }
        let _ = db.set_setting("launch_at_login.initialized", "1").await;
    });
}

pub fn run() {
    let builder = specta_builder();
    #[cfg(debug_assertions)]
    if let Err(e) = export_bindings() {
        eprintln!("could not export TypeScript bindings: {e}");
    }

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let db = tauri::async_runtime::block_on(ceevee_db::Db::open(&dir.join("ceevee.db")))?;
            let bus = ceevee_core::EventBus::new(256);
            scheduler::spawn(db.clone(), bus.clone());
            search_task::spawn(db.clone(), bus.clone());
            let mut registry = ceevee_agent::tools::Registry::new();
            ceevee_agent::builtin::register_all(&mut registry, &db)?;
            app.manage(registry);
            app.manage(bus);
            app.manage(db);

            tray::build(app)?;
            launch_at_login_default(app);
            if std::env::args().any(|a| a == "--minimized") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            Ok(())
        })
        // Closing the window hides to the tray; Quit is in the tray menu.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(builder.invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while building CeeVee");

    app.run(|_app, event| {
        if let RunEvent::ExitRequested {
            code: None, api, ..
        } = event
        {
            api.prevent_exit(); // keep running in the tray
        }
    });
}

/// True if the committed bindings file matches what the code would generate.
/// This is a command-line mode rather than a unit test: on Windows a test
/// executable that links Tauri lacks the application manifest and fails to
/// start (STATUS_ENTRYPOINT_NOT_FOUND).
pub fn bindings_are_current() -> Result<bool, Box<dyn std::error::Error>> {
    let tmp = std::env::temp_dir().join("ceevee-bindings-check.ts");
    specta_builder().export(specta_typescript::Typescript::default(), &tmp)?;
    let norm = |s: String| s.replace("\r\n", "\n");
    Ok(norm(std::fs::read_to_string(&tmp)?) == norm(std::fs::read_to_string(BINDINGS_PATH)?))
}
