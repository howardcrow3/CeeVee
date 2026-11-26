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
}

/// True when this build can send feedback (it carries a feedback key).
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
        env!("CARGO_PKG_VERSION"),
        platform,
    )?;
    feedback::send(body).await
}
