//! Read-only Gmail over the REST API.

use std::future::Future;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ceevee_core::{Error, Result};
use serde_json::Value;

const BASE: &str = "https://gmail.googleapis.com";

fn err(e: impl std::fmt::Display) -> Error {
    Error::Other(format!("Gmail: {e}"))
}

/// What the mail sync needs from a mailbox. Gmail implements it; tests use an in-memory one.
pub trait MailApi: Send + Sync {
    /// IDs of messages matching a Gmail search query, newest first, at most `max`.
    fn list_ids(&self, query: &str, max: usize)
        -> impl Future<Output = Result<Vec<String>>> + Send;
    /// Just the headers of a message, as bytes a mail parser can read.
    fn headers(&self, id: &str) -> impl Future<Output = Result<Vec<u8>>> + Send;
    /// The whole message.
    fn raw(&self, id: &str) -> impl Future<Output = Result<Vec<u8>>> + Send;
}

/// The reason and message in a Google API error body. Neither holds mail content.
fn google_error(body: &str) -> (String, String) {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let e = &v["error"];
    let reason = e["errors"][0]["reason"]
        .as_str()
        .or_else(|| e["status"].as_str())
        .unwrap_or("unknown");
    let message = e["message"].as_str().unwrap_or("no message");
    (
        reason.chars().take(60).collect(),
        message.chars().take(200).collect(),
    )
}

pub struct Gmail {
    http: reqwest::Client,
    base: String,
    token: String,
}

const HEADERS: [&str; 7] = [
    "From",
    "Reply-To",
    "Message-ID",
    "References",
    "In-Reply-To",
    "Subject",
    "Date",
];

impl Gmail {
    pub fn new(access_token: &str) -> Self {
        Self::with_base(access_token, BASE)
    }

    pub fn with_base(access_token: &str, base: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            base: base.trim_end_matches('/').into(),
            token: access_token.into(),
        }
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let mut attempt = 0u32;
        loop {
            let resp = self
                .http
                .get(format!("{}{path}", self.base))
                .bearer_auth(&self.token)
                .send()
                .await
                .map_err(err)?;
            let status = resp.status();
            let wait = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            let text = resp.text().await.map_err(err)?;
            if status.as_u16() == 401 {
                return Err(err("Google no longer allows CeeVee to read this mailbox. Connect Google again in Settings."));
            }
            if status.is_success() {
                return serde_json::from_str(&text).map_err(err);
            }
            let (reason, message) = google_error(&text);
            tracing::warn!(
                "gmail: request answered {status}, reason \"{reason}\", message \"{message}\" (attempt {})",
                attempt + 1
            );
            // Busy or rate-limited: wait and try again rather than give up the whole mail check.
            let busy = status.as_u16() == 429
                || status.is_server_error()
                || (status.as_u16() == 403
                    && (reason.contains("ate") && reason.contains("imit")
                        || reason.contains("uota")));
            if busy && attempt < 4 {
                let secs = wait.unwrap_or(1 << attempt).min(30);
                tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
                attempt += 1;
                continue;
            }
            return Err(err(format!(
                "the server answered {status}: {message} ({reason})"
            )));
        }
    }

    /// The address of the signed-in mailbox.
    pub async fn address(&self) -> Result<String> {
        let v = self.get("/gmail/v1/users/me/profile").await?;
        v["emailAddress"]
            .as_str()
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| err("no address in the profile"))
    }
}

impl MailApi for Gmail {
    async fn list_ids(&self, query: &str, max: usize) -> Result<Vec<String>> {
        let mut ids = vec![];
        let mut page: Option<String> = None;
        while ids.len() < max {
            let mut path = format!(
                "/gmail/v1/users/me/messages?maxResults={}&q={}",
                (max - ids.len()).min(100),
                crate::oauth::encode(query)
            );
            if let Some(p) = &page {
                path.push_str(&format!("&pageToken={}", crate::oauth::encode(p)));
            }
            let v = self.get(&path).await?;
            ids.extend(
                v["messages"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|m| m["id"].as_str().map(str::to_string)),
            );
            page = v["nextPageToken"].as_str().map(str::to_string);
            if page.is_none() {
                break;
            }
        }
        ids.truncate(max);
        Ok(ids)
    }

    async fn headers(&self, id: &str) -> Result<Vec<u8>> {
        let names = HEADERS
            .iter()
            .map(|h| format!("metadataHeaders={h}"))
            .collect::<Vec<_>>()
            .join("&");
        let v = self
            .get(&format!(
                "/gmail/v1/users/me/messages/{id}?format=metadata&{names}"
            ))
            .await?;
        let mut out = String::new();
        for h in v["payload"]["headers"].as_array().into_iter().flatten() {
            if let (Some(n), Some(val)) = (h["name"].as_str(), h["value"].as_str()) {
                out.push_str(&format!("{n}: {}\r\n", val.replace(['\r', '\n'], " ")));
            }
        }
        out.push_str("\r\n");
        // Gmail's own preview of the start of the message, so the first look has something to read.
        if let Some(snippet) = v["snippet"].as_str() {
            out.push_str(snippet);
            out.push_str("\r\n");
        }
        Ok(out.into_bytes())
    }

    async fn raw(&self, id: &str) -> Result<Vec<u8>> {
        let v = self
            .get(&format!("/gmail/v1/users/me/messages/{id}?format=raw"))
            .await?;
        let raw = v["raw"]
            .as_str()
            .ok_or_else(|| err("the message has no body"))?;
        URL_SAFE_NO_PAD
            .decode(raw.trim_end_matches('='))
            .map_err(err)
    }
}
