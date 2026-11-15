//! Trial and license commands for the UI. All rules live in `ceevee-license`.

use std::sync::Arc;

use ceevee_license::{
    anchor, api, key, machine, status::Problem, HttpApi, LicenseError, LicenseService, Status,
    Verifier,
};
use chrono::Utc;
use serde::Serialize;
use specta::Type;
use tauri::Manager;

pub type Service = LicenseService<HttpApi, Box<dyn ceevee_license::Anchor>>;

/// Build the service and, in the background, record the trial start and ask
/// turtlebox.studio for the trial token (quietly skipped when offline).
pub fn setup(app: &tauri::App, db: ceevee_db::Db) {
    let svc = Arc::new(LicenseService::new(
        db,
        HttpApi::default(),
        anchor::platform(),
        Verifier::compiled(),
        machine::current(),
        env!("CARGO_PKG_VERSION"),
    ));
    app.manage(svc.clone());
    tauri::async_runtime::spawn(async move {
        if let Err(e) = svc.status(Utc::now()).await {
            tracing::warn!("license status failed: {e}");
        }
        svc.fetch_trial_token().await;
    });
}

/// What the license screen and the banner need.
#[derive(Debug, Clone, Serialize, Type)]
pub struct LicenseView {
    /// `trial`, `expired` or `licensed`.
    pub state: String,
    pub days_left: Option<u32>,
    pub ends_at: Option<String>,
    pub key_hint: Option<String>,
    pub seats: Option<u32>,
    /// True when the app may only view and export.
    pub read_only: bool,
    /// Why a stored license was not used, in plain words.
    pub note: Option<String>,
}

fn view(r: ceevee_license::service::Report) -> LicenseView {
    let read_only = r.status.read_only();
    let note = r.problem.map(|p| {
        match p {
            Problem::BadSignature => LicenseError::BadSignature,
            Problem::WrongMachine => LicenseError::WrongMachine,
            Problem::WrongVersion => LicenseError::WrongVersion,
        }
        .to_string()
    });
    let mut v = LicenseView {
        state: "trial".into(),
        days_left: None,
        ends_at: None,
        key_hint: None,
        seats: None,
        read_only,
        note,
    };
    match r.status {
        Status::NotStarted => {}
        Status::Trial { ends_at, days_left } => {
            v.days_left = Some(days_left);
            v.ends_at = Some(ends_at.to_rfc3339());
        }
        Status::Expired { ended_at } => {
            v.state = "expired".into();
            v.ends_at = Some(ended_at.to_rfc3339());
        }
        Status::Licensed { key, seats } => {
            v.state = "licensed".into();
            v.key_hint = Some(key::hint(&key));
            v.seats = Some(seats);
        }
    }
    v
}

fn err(e: LicenseError) -> String {
    e.to_string()
}

/// Where the person stands: trial days left, expired (read-only) or licensed.
#[tauri::command]
#[specta::specta]
pub async fn license_status(svc: tauri::State<'_, Arc<Service>>) -> Result<LicenseView, String> {
    svc.status(Utc::now()).await.map(view).map_err(err)
}

/// Activate a license key on this computer.
#[tauri::command]
#[specta::specta]
pub async fn activate_license(
    svc: tauri::State<'_, Arc<Service>>,
    key: String,
) -> Result<LicenseView, String> {
    svc.activate(&key, Utc::now()).await.map(view).map_err(err)
}

/// Free this computer's seat and remove the license from it.
#[tauri::command]
#[specta::specta]
pub async fn deactivate_license(
    svc: tauri::State<'_, Arc<Service>>,
) -> Result<LicenseView, String> {
    svc.deactivate(Utc::now()).await.map(view).map_err(err)
}

/// Open the turtlebox.studio page in the browser: `buy` or `manage`. Only these two
/// fixed addresses can be opened.
#[tauri::command]
#[specta::specta]
pub fn open_license_page(app: tauri::AppHandle, which: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let url = match which.as_str() {
        "buy" => api::PURCHASE_URL,
        "manage" => api::MY_LICENSE_URL,
        _ => return Err("unknown page".into()),
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}
