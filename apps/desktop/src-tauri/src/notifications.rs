//! Desktop notification types and their defaults (TECHNICAL_PLAN 3.1).
//! Each type can be switched off with a `notify.<key>` setting ("on" / "off").

use ceevee_db::Db;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    NewMatches,
    WatchedCompanyPosted,
    EmployerReplied,
    InterviewReminder,
    FollowUpDue,
    ReadyForReview,
    RejectionReceived,
}

impl Kind {
    pub const ALL: [Kind; 7] = [
        Kind::NewMatches,
        Kind::WatchedCompanyPosted,
        Kind::EmployerReplied,
        Kind::InterviewReminder,
        Kind::FollowUpDue,
        Kind::ReadyForReview,
        Kind::RejectionReceived,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Kind::NewMatches => "new_matches",
            Kind::WatchedCompanyPosted => "watched_company_posted",
            Kind::EmployerReplied => "employer_replied",
            Kind::InterviewReminder => "interview_reminder",
            Kind::FollowUpDue => "follow_up_due",
            Kind::ReadyForReview => "ready_for_review",
            Kind::RejectionReceived => "rejection_received",
        }
    }

    /// Rejections are shown in the app only; everything else is on.
    pub fn default_on(self) -> bool {
        self != Kind::RejectionReceived
    }
}

/// Whether notifications of `kind` are enabled, falling back to the default.
pub async fn is_enabled(db: &Db, kind: Kind) -> bool {
    match db.get_setting(&format!("notify.{}", kind.key())).await {
        Ok(Some(v)) => v != "off",
        _ => kind.default_on(),
    }
}

/// Show a notification if its type is enabled. Failures are logged, not raised.
pub async fn notify(app: &AppHandle, db: &Db, kind: Kind, title: &str, body: &str) {
    if !is_enabled(db, kind).await {
        return;
    }
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!("notification failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn defaults_and_overrides() {
        let db = Db::open_in_memory().await.unwrap();
        for k in Kind::ALL {
            assert_eq!(
                is_enabled(&db, k).await,
                k != Kind::RejectionReceived,
                "{k:?}"
            );
        }
        db.set_setting("notify.new_matches", "off").await.unwrap();
        db.set_setting("notify.rejection_received", "on")
            .await
            .unwrap();
        assert!(!is_enabled(&db, Kind::NewMatches).await);
        assert!(is_enabled(&db, Kind::RejectionReceived).await);
    }

    #[test]
    fn keys_are_unique() {
        let mut keys: Vec<_> = Kind::ALL.iter().map(|k| k.key()).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), Kind::ALL.len());
    }
}
