//! CeeVee desktop shell: window, tray, single instance and the typed command
//! surface the UI calls. Domain logic lives in the `ceevee-*` crates.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
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
        .commands(tauri_specta::collect_commands![app_info])
        .typ::<ceevee_core::Event>()
}

pub const BINDINGS_PATH: &str = "../ui/src/lib/bindings.ts";

/// Write `ui/src/lib/bindings.ts`. Run from `apps/desktop/src-tauri`:
/// `cargo run -- --export-bindings`.
pub fn export_bindings() -> Result<(), Box<dyn std::error::Error>> {
    specta_builder().export(specta_typescript::Typescript::default(), BINDINGS_PATH)?;
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn run() {
    let builder = specta_builder();
    #[cfg(debug_assertions)]
    if let Err(e) = export_bindings() {
        eprintln!("could not export TypeScript bindings: {e}");
    }

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let db = tauri::async_runtime::block_on(ceevee_db::Db::open(&dir.join("ceevee.db")))?;
            app.manage(db);

            let open = MenuItem::with_id(app, "open", "Open CeeVee", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let mut tray =
                TrayIconBuilder::new()
                    .menu(&menu)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "open" => show_main_window(app),
                        "quit" => app.exit(0),
                        _ => {}
                    });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
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
