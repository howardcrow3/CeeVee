//! "Send feedback" commands. The rules and the signing live in `ceevee_license::feedback`.

use ceevee_license::feedback;
use serde::Deserialize;
use specta::Type;

/// What the user filled in on the feedback form.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct FeedbackInput {
    /// 1 to 5 stars, or none.
    pub rating: Option<u8>,
    pub message: String,
    /// `feature`, `issue`, `praise` or `other`.
    pub category: Option<String>,
    /// Only when the user wants a reply.
    pub email: Option<String>,
    /// "Send anonymously": no email is sent, and the service keeps none.
    #[serde(default)]
    pub anonymous: bool,
}

/// The address to prefill on the form: the profile's email, or else the first connected mail
/// account. `None` when CeeVee knows neither. (Async commands that borrow state must return a
/// `Result`; this one never fails.)
#[tauri::command]
#[specta::specta]
pub async fn feedback_email(db: tauri::State<'_, ceevee_db::Db>) -> Result<Option<String>, String> {
    let usable = |e: String| Some(e.trim().to_string()).filter(|e| feedback::email_ok(e));
    let profile = ceevee_profile::repo::get_basics(&db)
        .await
        .ok()
        .and_then(|b| b.email);
    if let Some(e) = profile.and_then(usable) {
        return Ok(Some(e));
    }
    let accounts = ceevee_mail::store::list_accounts(&db)
        .await
        .unwrap_or_default();
    Ok(accounts.into_iter().map(|a| a.address).find_map(usable))
}

/// True when this build can send feedback (always: requests are signed per install).
#[tauri::command]
#[specta::specta]
pub fn feedback_available() -> bool {
    feedback::available()
}

#[tauri::command]
#[specta::specta]
pub async fn send_feedback(input: FeedbackInput) -> Result<(), String> {
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    let body = feedback::body(
        input.rating,
        &input.message,
        input.category.as_deref(),
        input.email.as_deref(),
        input.anonymous,
        env!("CARGO_PKG_VERSION"),
        platform,
    )?;
    feedback::send(body).await
}
