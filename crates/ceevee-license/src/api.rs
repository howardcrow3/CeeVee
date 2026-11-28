//! The turtlebox.studio client. The server side lives in the highturtle repo; the
//! contract is written down in docs/LICENSE_API.md.

use serde::{Deserialize, Serialize};

use crate::{LicenseError, Result};

/// Base address of the CeeVee license API.
pub const DEFAULT_BASE: &str = "https://turtlebox.studio/api/ceevee/v1";
/// Where "Buy CeeVee" sends people (Paddle checkout is on that page).
pub const PURCHASE_URL: &str = "https://turtlebox.studio/ceevee/buy?from=app";
/// Where people free a seat or find their key again.
pub const MY_LICENSE_URL: &str = "https://turtlebox.studio/ceevee/license";

/// Public pages linked from the app (text in docs/customer/).
pub const PRIVACY_URL: &str = "https://turtlebox.studio/ceevee/privacy";
pub const TERMS_URL: &str = "https://turtlebox.studio/ceevee/terms";
pub const HELP_URL: &str = "https://turtlebox.studio/ceevee/help";

#[allow(async_fn_in_trait)]
pub trait Api: Send + Sync {
    /// First contact: a signed trial token for this machine.
    async fn trial_token(&self, machine: &str) -> Result<String>;
    /// Exchange a key for a signed license file for this machine.
    async fn activate(&self, key: &str, machine: &str, app_version: &str) -> Result<String>;
    /// Give this machine's seat back.
    async fn deactivate(&self, key: &str, machine: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct HttpApi {
    base: String,
    http: reqwest::Client,
}

impl HttpApi {
    pub fn new(base: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .user_agent(concat!("CeeVee/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("http client builds");
        Self {
            base: base.into().trim_end_matches('/').to_string(),
            http,
        }
    }
}

impl Default for HttpApi {
    fn default() -> Self {
        Self::new(DEFAULT_BASE)
    }
}

#[derive(Serialize)]
struct TrialReq<'a> {
    machine: &'a str,
}
#[derive(Serialize)]
struct ActivateReq<'a> {
    key: &'a str,
    machine: &'a str,
    app_version: &'a str,
    platform: &'a str,
}
#[derive(Serialize)]
struct DeactivateReq<'a> {
    key: &'a str,
    machine: &'a str,
}
#[derive(Deserialize)]
struct TrialResp {
    token: String,
}
#[derive(Deserialize)]
struct ActivateResp {
    license: String,
}
#[derive(Deserialize, Default)]
struct ErrorBody {
    #[serde(default)]
    error: String,
    #[serde(default)]
    message: String,
}

/// Map a failed response to a [`LicenseError`].
fn map_error(status: u16, body: &str) -> LicenseError {
    let b: ErrorBody = serde_json::from_str(body).unwrap_or_default();
    match (status, b.error.as_str()) {
        (_, "seats_full") | (409, _) => LicenseError::SeatsFull,
        (_, "unknown_key") | (404, _) => LicenseError::UnknownKey,
        (_, "invalid_key") => LicenseError::UnknownKey,
        _ if b.message.is_empty() => LicenseError::Rejected(format!("HTTP {status}")),
        _ => LicenseError::Rejected(b.message),
    }
}

impl HttpApi {
    async fn post<B: Serialize>(&self, path: &str, body: &B) -> Result<(u16, String)> {
        let res = self
            .http
            .post(format!("{}/{path}", self.base))
            .header("content-type", "application/json")
            .body(serde_json::to_vec(body).map_err(|_| LicenseError::Network)?)
            .send()
            .await
            .map_err(|e| {
                tracing::info!("license request failed: {e}");
                LicenseError::Network
            })?;
        let status = res.status().as_u16();
        let text = res.text().await.map_err(|_| LicenseError::Network)?;
        Ok((status, text))
    }
}

impl Api for HttpApi {
    async fn trial_token(&self, machine: &str) -> Result<String> {
        let (status, text) = self.post("trial", &TrialReq { machine }).await?;
        if !(200..300).contains(&status) {
            return Err(map_error(status, &text));
        }
        serde_json::from_str::<TrialResp>(&text)
            .map(|r| r.token)
            .map_err(|_| LicenseError::Rejected("unexpected reply".into()))
    }

    async fn activate(&self, key: &str, machine: &str, app_version: &str) -> Result<String> {
        let platform = if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "other"
        };
        let (status, text) = self
            .post(
                "activate",
                &ActivateReq {
                    key,
                    machine,
                    app_version,
                    platform,
                },
            )
            .await?;
        if !(200..300).contains(&status) {
            return Err(map_error(status, &text));
        }
        serde_json::from_str::<ActivateResp>(&text)
            .map(|r| r.license)
            .map_err(|_| LicenseError::Rejected("unexpected reply".into()))
    }

    async fn deactivate(&self, key: &str, machine: &str) -> Result<()> {
        let (status, text) = self
            .post("deactivate", &DeactivateReq { key, machine })
            .await?;
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(map_error(status, &text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn error_mapping() {
        assert!(matches!(
            map_error(409, r#"{"error":"seats_full"}"#),
            LicenseError::SeatsFull
        ));
        assert!(matches!(map_error(404, ""), LicenseError::UnknownKey));
        assert!(matches!(
            map_error(400, r#"{"error":"invalid_key"}"#),
            LicenseError::UnknownKey
        ));
        assert!(
            matches!(map_error(500, r#"{"message":"boom"}"#), LicenseError::Rejected(m) if m == "boom")
        );
        assert!(matches!(map_error(502, "<html>"), LicenseError::Rejected(m) if m == "HTTP 502"));
    }

    /// One-shot HTTP server on localhost; returns its base URL and the request it saw.
    async fn serve(
        status: &'static str,
        body: &'static str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        let h = tokio::spawn(async move {
            let (mut s, _) = l.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let mut got = Vec::new();
            loop {
                let n = s.read(&mut buf).await.unwrap();
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got).to_string();
                if let Some((head, rest)) = text.split_once("\r\n\r\n") {
                    let len = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if rest.len() >= len {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let resp = format!("HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
            s.write_all(resp.as_bytes()).await.unwrap();
            String::from_utf8_lossy(&got).to_string()
        });
        (base, h)
    }

    #[tokio::test]
    async fn activate_sends_key_and_machine_and_returns_the_license() {
        let (base, seen) = serve("200 OK", r#"{"license":"abc.def"}"#).await;
        let api = HttpApi::new(base);
        let lic = api.activate("CEEV-A", "mach", "0.1.0").await.unwrap();
        assert_eq!(lic, "abc.def");
        let req = seen.await.unwrap();
        assert!(req.starts_with("POST /activate "), "{req}");
        assert!(req.contains(r#""key":"CEEV-A""#) && req.contains(r#""machine":"mach""#));
    }

    #[tokio::test]
    async fn seat_limit_reply_becomes_seats_full() {
        let (base, _) = serve("409 Conflict", r#"{"error":"seats_full"}"#).await;
        let err = HttpApi::new(base)
            .activate("k", "m", "1")
            .await
            .unwrap_err();
        assert!(matches!(err, LicenseError::SeatsFull));
    }

    #[tokio::test]
    async fn unreachable_server_is_a_network_error() {
        let err = HttpApi::new("http://127.0.0.1:1")
            .trial_token("m")
            .await
            .unwrap_err();
        assert!(matches!(err, LicenseError::Network));
    }
}
