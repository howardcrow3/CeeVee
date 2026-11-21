//! Opt-in, anonymous usage analytics.
//!
//! Privacy rules, enforced by the types here rather than by care:
//! - Nothing is sent unless the person chose "Share" ([`Consent::Granted`]). The default is
//!   [`Consent::Unset`], which sends nothing.
//! - An [`Event`] is a fixed list of names with fixed, non-text parameters. There is no way to
//!   attach a free-form string, so resume, job, email or chat text cannot be added by mistake.
//! - The only identifier is a random id made for analytics alone. It is not the license machine
//!   hash, not the license key, and not derived from anything on the computer.

use std::time::Duration;

use ceevee_db::Db;
use serde_json::{json, Value};

const CONSENT_KEY: &str = "analytics_consent";
const ID_KEY: &str = "analytics_client_id";
const ENDPOINT: &str = "https://www.google-analytics.com/mp/collect";

/// The person's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    /// Not asked yet. Nothing is sent.
    Unset,
    Granted,
    Denied,
}

impl Consent {
    fn parse(s: Option<&str>) -> Self {
        match s {
            Some("granted") => Self::Granted,
            Some("denied") => Self::Denied,
            _ => Self::Unset,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "denied",
            Self::Unset => "unset",
        }
    }
}

/// Features worth counting. Names only; adding a variant is the only way to add an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    ResumeImported,
    JobSearchRun,
    ApplicationDrafted,
    ApplicationSubmitted,
    MailChecked,
    DocumentExported,
    GoogleSignedIn,
}

impl Feature {
    fn name(self) -> &'static str {
        match self {
            Self::ResumeImported => "resume_imported",
            Self::JobSearchRun => "job_search_run",
            Self::ApplicationDrafted => "application_drafted",
            Self::ApplicationSubmitted => "application_submitted",
            Self::MailChecked => "mail_checked",
            Self::DocumentExported => "document_exported",
            Self::GoogleSignedIn => "google_signed_in",
        }
    }
}

/// Everything that can be sent. No variant carries text from the person's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// App started. Version, OS and architecture are build constants.
    AppStart,
    /// A feature was used.
    FeatureUsed(Feature),
    /// The app closed unexpectedly. Carries no message, path or stack text.
    Crash,
}

/// Build facts attached to every event.
#[derive(Debug, Clone)]
pub struct Context {
    pub version: String,
    pub os: String,
}

impl Event {
    fn to_json(&self, ctx: &Context) -> Value {
        let (name, extra) = match self {
            Self::AppStart => ("app_start", None),
            Self::FeatureUsed(f) => ("feature_used", Some(("feature", f.name()))),
            Self::Crash => ("app_crash", None),
        };
        let mut params = json!({
            "app_version": ctx.version,
            "os": ctx.os,
            "engagement_time_msec": 1,
        });
        if let Some((k, v)) = extra {
            params[k] = json!(v);
        }
        json!({ "name": name, "params": params })
    }
}

/// Where events go. `None` credentials means analytics is built out of this build.
#[derive(Debug, Clone)]
pub struct Target {
    pub measurement_id: String,
    pub api_secret: String,
    /// Overridable for tests.
    pub endpoint: String,
}

impl Target {
    pub fn new(measurement_id: &str, api_secret: &str) -> Self {
        Self {
            measurement_id: measurement_id.into(),
            api_secret: api_secret.into(),
            endpoint: ENDPOINT.into(),
        }
    }
}

pub struct Analytics {
    db: Db,
    target: Option<Target>,
    ctx: Context,
    http: reqwest::Client,
}

impl Analytics {
    pub fn new(db: Db, target: Option<Target>, ctx: Context) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        Self {
            db,
            target,
            ctx,
            http,
        }
    }

    pub async fn consent(&self) -> Consent {
        let v = self.db.get_setting(CONSENT_KEY).await.ok().flatten();
        Consent::parse(v.as_deref())
    }

    /// Record the choice. Turning analytics off also forgets the random id, so a later
    /// opt-in starts a fresh, unlinked one.
    pub async fn set_consent(&self, c: Consent) -> ceevee_core::Result<()> {
        self.db.set_setting(CONSENT_KEY, c.as_str()).await?;
        if c != Consent::Granted {
            self.db.set_setting(ID_KEY, "").await?;
        }
        Ok(())
    }

    async fn client_id(&self) -> Option<String> {
        if let Some(id) = self.db.get_setting(ID_KEY).await.ok().flatten() {
            if !id.is_empty() {
                return Some(id);
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.db.set_setting(ID_KEY, &id).await.ok()?;
        Some(id)
    }

    /// Send one event if, and only if, the person agreed and this build has credentials.
    /// Never fails the caller: errors are logged and dropped.
    pub async fn track(&self, event: Event) {
        let Some(target) = &self.target else { return };
        if self.consent().await != Consent::Granted {
            return;
        }
        let Some(client_id) = self.client_id().await else {
            return;
        };
        let body = json!({ "client_id": client_id, "events": [event.to_json(&self.ctx)] });
        let result = self
            .http
            .post(&target.endpoint)
            .query(&[
                ("measurement_id", target.measurement_id.as_str()),
                ("api_secret", target.api_secret.as_str()),
            ])
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await;
        if let Err(e) = result {
            tracing::debug!("analytics send failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests;
