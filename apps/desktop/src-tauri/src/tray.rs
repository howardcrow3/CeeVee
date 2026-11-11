//! System tray (Windows) and menu bar (macOS) menu. TECHNICAL_PLAN 3.1.

use ceevee_core::{Event, EventBus};
use ceevee_db::Db;
use chrono::{Duration, Local, NaiveTime, TimeZone};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::show_main_window;

const PAUSED_UNTIL: &str = "background.paused_until";
/// "Until resumed": far enough ahead to never expire on its own.
const FAR_FUTURE_DAYS: i64 = 36_500;

/// When "Pause until tomorrow" ends: 7:00 tomorrow, local time.
fn tomorrow_morning() -> chrono::DateTime<Local> {
    let t = Local::now().date_naive().succ_opt().unwrap_or_default();
    let at = t.and_time(NaiveTime::from_hms_opt(7, 0, 0).unwrap_or_default());
    Local
        .from_local_datetime(&at)
        .earliest()
        .unwrap_or_else(|| Local::now() + Duration::hours(12))
}

fn pause_until(id: &str) -> Option<Option<chrono::DateTime<Local>>> {
    match id {
        "pause_1h" => Some(Some(Local::now() + Duration::hours(1))),
        "pause_tomorrow" => Some(Some(tomorrow_morning())),
        "pause_forever" => Some(Some(Local::now() + Duration::days(FAR_FUTURE_DAYS))),
        "resume" => Some(None),
        _ => None,
    }
}

pub fn build(app: &tauri::App) -> tauri::Result<()> {
    // Counts are placeholders until search and approvals exist (shown disabled).
    let open = MenuItem::with_id(app, "open", "Open CeeVee", true, None::<&str>)?;
    let new_jobs = MenuItem::with_id(app, "new_jobs", "New jobs today: 0", false, None::<&str>)?;
    let approvals = MenuItem::with_id(
        app,
        "approvals",
        "Approvals waiting: 0",
        false,
        None::<&str>,
    )?;
    let run_now = MenuItem::with_id(app, "run_searches", "Run searches now", true, None::<&str>)?;
    let pause = Submenu::with_items(
        app,
        "Pause background work",
        true,
        &[
            &MenuItem::with_id(app, "pause_1h", "For 1 hour", true, None::<&str>)?,
            &MenuItem::with_id(app, "pause_tomorrow", "Until tomorrow", true, None::<&str>)?,
            &MenuItem::with_id(app, "pause_forever", "Until resumed", true, None::<&str>)?,
            &MenuItem::with_id(app, "resume", "Resume now", true, None::<&str>)?,
        ],
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit CeeVee", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &open, &sep, &new_jobs, &approvals, &run_now, &pause, &sep, &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(|app: &AppHandle, event| {
            let id = event.id.as_ref();
            match id {
                "open" => show_main_window(app),
                "quit" => app.exit(0),
                "run_searches" => {
                    if let Some(bus) = app.try_state::<EventBus>() {
                        bus.publish(Event::TaskDue {
                            task: "search.daily".into(),
                        });
                    }
                }
                _ => {
                    if let Some(until) = pause_until(id) {
                        let db = app.state::<Db>().inner().clone();
                        tauri::async_runtime::spawn(async move {
                            let value = until.map(|d| d.to_rfc3339()).unwrap_or_default();
                            if let Err(e) = db.set_setting(PAUSED_UNTIL, &value).await {
                                tracing::warn!("could not save pause: {e}");
                            }
                        });
                    }
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
