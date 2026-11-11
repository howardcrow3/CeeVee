//! CeeVee desktop shell: window, tray, single instance and the typed command
//! surface the UI calls. Domain logic lives in the `ceevee-*` crates.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

#[derive(serde::Serialize)]
struct AppInfo {
    name: &'static str,
    version: &'static str,
    schema_version: i64,
}

#[tauri::command]
async fn app_info(db: tauri::State<'_, ceevee_db::Db>) -> Result<AppInfo, String> {
    Ok(AppInfo {
        name: "CeeVee",
        version: env!("CARGO_PKG_VERSION"),
        schema_version: db.schema_version().await.map_err(|e| e.to_string())?,
    })
}

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn run() {
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
        .invoke_handler(tauri::generate_handler![app_info])
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
