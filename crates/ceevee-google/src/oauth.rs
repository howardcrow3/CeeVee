//! OAuth 2 for an installed app: authorization code with PKCE and a loopback redirect.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ceevee_core::{Error, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub const GMAIL_READONLY: &str = "https://www.googleapis.com/auth/gmail.readonly";
pub const SIGN_IN_SCOPES: [&str; 3] = ["openid", "email", "profile"];

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";

/// How long to wait for the person to finish signing in (they can also cancel).
pub const SIGN_IN_WAIT: std::time::Duration = std::time::Duration::from_secs(300);

fn err(e: impl std::fmt::Display) -> Error {
    Error::Other(format!("Google sign-in: {e}"))
}

#[derive(Debug, Clone)]
pub struct OAuthClient {
    pub client_id: String,
    /// Google issues a secret to desktop clients, but it is not confidential.
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub revoke_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    /// The signed-in address, from the ID token.
    pub email: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

fn random_token(bytes: usize) -> Result<String> {
    let mut b = vec![0u8; bytes];
    getrandom::fill(&mut b).map_err(err)?;
    Ok(URL_SAFE_NO_PAD.encode(b))
}

pub fn pkce() -> Result<Pkce> {
    let verifier = random_token(48)?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    Ok(Pkce {
        verifier,
        challenge,
    })
}

pub fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match std::str::from_utf8(&b[i + 1..i + 3])
                    .ok()
                    .and_then(|h| u8::from_str_radix(h, 16).ok())
                {
                    Some(v) => {
                        out.push(v);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Value of `name` in a query string.
pub fn query_param(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == name).then(|| decode(v))
    })
}

#[derive(Deserialize)]
struct TokenReply {
    access_token: String,
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: u64,
    id_token: Option<String>,
    #[serde(default)]
    scope: String,
}

/// The `email` claim of an ID token. The token came straight from Google's token endpoint over
/// TLS, so its signature is not checked again here.
fn email_from_id_token(id_token: &str) -> Option<String> {
    let payload = id_token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("email")?.as_str().map(str::to_ascii_lowercase)
}

impl OAuthClient {
    pub fn google(client_id: &str, client_secret: Option<&str>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.map(str::to_string),
            auth_url: AUTH_URL.into(),
            token_url: TOKEN_URL.into(),
            revoke_url: REVOKE_URL.into(),
        }
    }

    pub fn authorization_url(
        &self,
        redirect: &str,
        challenge: &str,
        state: &str,
        scopes: &[&str],
    ) -> String {
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}\
             &code_challenge_method=S256&state={}&access_type=offline&prompt=consent",
            self.auth_url,
            encode(&self.client_id),
            encode(redirect),
            encode(&scopes.join(" ")),
            challenge,
            state
        )
    }

    async fn token_request(&self, mut form: Vec<(&str, String)>) -> Result<TokenReply> {
        form.push(("client_id", self.client_id.clone()));
        if let Some(s) = &self.client_secret {
            form.push(("client_secret", s.clone()));
        }
        let body = form
            .iter()
            .map(|(k, v)| format!("{k}={}", encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let resp = reqwest::Client::new()
            .post(&self.token_url)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(err)?;
        let status = resp.status();
        let text = resp.text().await.map_err(err)?;
        if !status.is_success() {
            let why = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| {
                    v["error_description"]
                        .as_str()
                        .or(v["error"].as_str())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| status.to_string());
            return Err(err(why));
        }
        serde_json::from_str(&text).map_err(err)
    }

    pub async fn exchange(&self, code: &str, verifier: &str, redirect: &str) -> Result<Tokens> {
        let r = self
            .token_request(vec![
                ("grant_type", "authorization_code".into()),
                ("code", code.into()),
                ("code_verifier", verifier.into()),
                ("redirect_uri", redirect.into()),
            ])
            .await?;
        Ok(Tokens {
            email: r.id_token.as_deref().and_then(email_from_id_token),
            scopes: r
                .scope
                .split(' ')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
            access_token: r.access_token,
            refresh_token: r.refresh_token,
            expires_in: r.expires_in,
        })
    }

    /// Tell Google to stop honouring a token, so CeeVee no longer has access to the account.
    /// An already-revoked or expired token counts as success.
    pub async fn revoke(&self, token: &str) -> Result<()> {
        let resp = reqwest::Client::new()
            .post(&self.revoke_url)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(format!("token={}", encode(token)))
            .send()
            .await
            .map_err(err)?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let text = resp.text().await.unwrap_or_default();
        if text.contains("invalid_token") {
            return Ok(());
        }
        Err(err(format!(
            "Google did not confirm the sign-out ({status})"
        )))
    }

    /// A fresh access token from a saved refresh token.
    pub async fn refresh(&self, refresh_token: &str) -> Result<String> {
        let r = self
            .token_request(vec![
                ("grant_type", "refresh_token".into()),
                ("refresh_token", refresh_token.into()),
            ])
            .await?;
        Ok(r.access_token)
    }

    /// Run the whole sign-in: listen on a free local port, hand the sign-in address to
    /// `open_browser`, wait for Google to send the person back, and exchange the code.
    pub async fn sign_in(
        &self,
        scopes: &[&str],
        open_browser: impl FnOnce(&str),
    ) -> Result<Tokens> {
        self.sign_in_cancellable(
            scopes,
            open_browser,
            &tokio::sync::Notify::new(),
            SIGN_IN_WAIT,
        )
        .await
    }

    /// Like [`sign_in`](Self::sign_in), but gives up when `cancel` is notified or `wait` runs out.
    /// Either way the local port is released, so signing in again works straight away.
    pub async fn sign_in_cancellable(
        &self,
        scopes: &[&str],
        open_browser: impl FnOnce(&str),
        cancel: &tokio::sync::Notify,
        wait: std::time::Duration,
    ) -> Result<Tokens> {
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(err)?;
        let port = listener.local_addr().map_err(err)?.port();
        let redirect = format!("http://127.0.0.1:{port}");
        let p = pkce()?;
        let state = random_token(16)?;
        open_browser(&self.authorization_url(&redirect, &p.challenge, &state, scopes));
        let code = tokio::select! {
            r = tokio::time::timeout(wait, wait_for_code(&listener, &state)) => r
                .map_err(|_| err("timed out waiting for you to finish signing in. Choose Sign in with Google to try again"))??,
            () = cancel.notified() => return Err(err("sign-in cancelled")),
        };
        drop(listener);
        self.exchange(&code, &p.verifier, &redirect).await
    }
}

// Self-contained pages (fonts and mark inlined); regenerate with scripts/build_oauth_pages.py.
const DONE_PAGE: &str = include_str!("pages/oauth_done.html");
const DENIED_PAGE: &str = include_str!("pages/oauth_denied.html");
const ERROR_PAGE: &str = include_str!("pages/oauth_error.html");

/// Accept connections until one carries the redirect for `state`; answer it and return the code.
async fn wait_for_code(listener: &TcpListener, state: &str) -> Result<String> {
    loop {
        let (mut sock, _) = listener.accept().await.map_err(err)?;
        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await.map_err(err)?;
        let head = String::from_utf8_lossy(&buf[..n]).into_owned();
        let target = head.split_whitespace().nth(1).unwrap_or("");
        let query = target.split_once('?').map_or("", |(_, q)| q);
        let reply = |status: &str, body: &str| {
            format!(
                "HTTP/1.1 {status}\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            )
        };
        if query_param(query, "state").as_deref() != Some(state) {
            // Something else on this computer poked the port; ignore it and keep waiting.
            let _ = sock
                .write_all(reply("400 Bad Request", "Not the sign-in.").as_bytes())
                .await;
            continue;
        }
        if let Some(e) = query_param(query, "error") {
            let page = if e == "access_denied" {
                DENIED_PAGE
            } else {
                ERROR_PAGE
            };
            let _ = sock.write_all(reply("200 OK", page).as_bytes()).await;
            return Err(err(if e == "access_denied" {
                "you chose not to allow access".to_string()
            } else {
                e
            }));
        }
        let Some(code) = query_param(query, "code") else {
            let _ = sock
                .write_all(reply("400 Bad Request", "No code.").as_bytes())
                .await;
            continue;
        };
        let _ = sock.write_all(reply("200 OK", DONE_PAGE).as_bytes()).await;
        return Ok(code);
    }
}
