//! Read mail over IMAP with a password (an app password for Gmail and most providers).
//!
//! Only the INBOX is read, nothing is changed on the server (`BODY.PEEK`), and bodies are
//! downloaded only for messages the candidate filter wants. The first sync looks back 90 days;
//! later syncs ask for UIDs above the last one seen, unless the server says the UIDs were reset.

use std::collections::HashMap;
use std::sync::Arc;

use async_imap::Client;
use ceevee_core::{Error, Result};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

use crate::parse::{parse, ParsedMail};

/// How far back the first sync reads.
pub const FIRST_SYNC_DAYS: i64 = 90;
/// Most messages kept from one sync, so a huge inbox cannot stall the app.
pub const MAX_PER_SYNC: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    /// `false` only for a mail server on this computer (tests).
    #[serde(default = "yes")]
    pub tls: bool,
}

fn yes() -> bool {
    true
}

/// Where the last sync stopped.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncState {
    pub uid_validity: Option<u32>,
    pub last_uid: Option<u32>,
}

#[derive(Debug, Default)]
pub struct Fetched {
    /// Full raw messages of the candidates.
    pub messages: Vec<Vec<u8>>,
    pub state: SyncState,
    /// Messages seen, candidates or not.
    pub seen: usize,
}

/// Server settings for the big providers, from the part of the address after the `@`.
pub fn preset_for(address: &str) -> Option<(&'static str, u16)> {
    let domain = crate::parse::domain_of(address)?;
    Some(match domain.as_str() {
        "gmail.com" | "googlemail.com" => ("imap.gmail.com", 993),
        "outlook.com" | "hotmail.com" | "live.com" | "msn.com" => ("outlook.office365.com", 993),
        "yahoo.com" | "ymail.com" => ("imap.mail.yahoo.com", 993),
        "icloud.com" | "me.com" | "mac.com" => ("imap.mail.me.com", 993),
        "aol.com" => ("imap.aol.com", 993),
        "fastmail.com" => ("imap.fastmail.com", 993),
        _ => return None,
    })
}

fn imap_err(e: impl std::fmt::Display) -> Error {
    Error::Other(format!("mail server: {e}"))
}

/// Plain words for the errors people hit most often.
pub fn explain(e: &Error) -> String {
    let text = e.to_string();
    let lower = text.to_ascii_lowercase();
    if lower.contains("authenticat")
        || lower.contains("login")
        || lower.contains("invalid credentials")
        || lower.contains("auth")
    {
        "The mail server refused the sign-in. Check the address and use an app password rather than your normal password.".into()
    } else if lower.contains("dns")
        || lower.contains("resolve")
        || lower.contains("connect")
        || lower.contains("timed out")
    {
        "CeeVee could not reach the mail server. Check the server name, the port and your connection.".into()
    } else {
        text
    }
}

fn tls_connector() -> TlsConnector {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder_with_provider(Arc::new(
        tokio_rustls::rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map(|b| b.with_root_certificates(roots).with_no_client_auth());
    TlsConnector::from(Arc::new(
        config.expect("ring supports the default TLS versions"),
    ))
}

/// `15-Aug-2026` for the day `days` before `now_secs`.
pub fn imap_date(now_secs: i64, days: i64) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let iso = crate::pipeline::unix_to_iso(now_secs - days * 86_400);
    let month: usize = iso[5..7].parse().unwrap_or(1);
    let day: u32 = iso[8..10].parse().unwrap_or(1);
    format!(
        "{day}-{}-{}",
        MONTHS[month.saturating_sub(1).min(11)],
        &iso[..4]
    )
}

/// Connect, read new mail, return the candidates. `want` decides from the headers alone whether
/// a message's body is worth downloading.
pub async fn fetch_new(
    cfg: &ImapConfig,
    password: &str,
    state: &SyncState,
    now_secs: i64,
    want: impl Fn(&ParsedMail) -> bool,
) -> Result<Fetched> {
    let tcp = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        TcpStream::connect((cfg.host.as_str(), cfg.port)),
    )
    .await
    .map_err(|_| imap_err("timed out connecting"))?
    .map_err(|e| imap_err(format!("could not connect: {e}")))?;
    if cfg.tls {
        let name = ServerName::try_from(cfg.host.clone()).map_err(imap_err)?;
        let tls = tls_connector().connect(name, tcp).await.map_err(imap_err)?;
        fetch_over(tls, cfg, password, state, now_secs, want).await
    } else {
        fetch_over(tcp, cfg, password, state, now_secs, want).await
    }
}

/// Check a sign-in without reading anything.
pub async fn test_login(cfg: &ImapConfig, password: &str) -> Result<()> {
    fetch_new(
        cfg,
        password,
        &SyncState {
            uid_validity: None,
            last_uid: Some(u32::MAX - 1),
        },
        0,
        |_| false,
    )
    .await
    .map(|_| ())
}

async fn fetch_over<T>(
    stream: T,
    cfg: &ImapConfig,
    password: &str,
    state: &SyncState,
    now_secs: i64,
    want: impl Fn(&ParsedMail) -> bool,
) -> Result<Fetched>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + std::fmt::Debug,
{
    let mut client = Client::new(stream);
    client
        .read_response()
        .await
        .map_err(imap_err)?
        .ok_or_else(|| imap_err("the server closed the connection"))?;
    let mut session = client
        .login(&cfg.username, password)
        .await
        .map_err(|(e, _)| imap_err(format!("authentication failed: {e}")))?;
    let mailbox = session.select("INBOX").await.map_err(imap_err)?;
    let validity = mailbox.uid_validity;

    let reset = state.uid_validity != validity || state.last_uid.is_none();
    let query = if reset {
        format!("SINCE {}", imap_date(now_secs, FIRST_SYNC_DAYS))
    } else {
        format!("UID {}:*", state.last_uid.unwrap_or(0) + 1)
    };
    let floor = if reset {
        0
    } else {
        state.last_uid.unwrap_or(0)
    };
    let mut uids: Vec<u32> = session
        .uid_search(&query)
        .await
        .map_err(imap_err)?
        .into_iter()
        .filter(|u| *u > floor)
        .collect();
    uids.sort_unstable();
    if uids.len() > MAX_PER_SYNC {
        if reset {
            // A first sync starts with the newest mail.
            uids = uids.split_off(uids.len() - MAX_PER_SYNC);
        } else {
            // Catching up after a gap works oldest first and remembers where it stopped, so
            // the rest is read on the next sync instead of being skipped.
            uids.truncate(MAX_PER_SYNC);
        }
    }

    let last_uid = match uids.last() {
        Some(top) => Some(*top),
        None if reset => Some(mailbox.uid_next.map_or(0, |n| n.saturating_sub(1))),
        None => state.last_uid,
    };
    let mut out = Fetched {
        state: SyncState {
            uid_validity: validity,
            last_uid,
        },
        ..Default::default()
    };
    out.seen = uids.len();

    let mut wanted: Vec<u32> = vec![];
    for chunk in uids.chunks(100) {
        let set = chunk
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let stream = session
            .uid_fetch(&set, "(UID BODY.PEEK[HEADER])")
            .await
            .map_err(imap_err)?;
        let fetches: Vec<_> = stream.try_collect().await.map_err(imap_err)?;
        for f in fetches {
            let (Some(uid), Some(header)) = (f.uid, f.header()) else {
                continue;
            };
            if parse(header).map(|m| want(&m)).unwrap_or(false) {
                wanted.push(uid);
            }
        }
    }
    let mut bodies: HashMap<u32, Vec<u8>> = HashMap::new();
    for chunk in wanted.chunks(20) {
        let set = chunk
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let stream = session
            .uid_fetch(&set, "(UID BODY.PEEK[])")
            .await
            .map_err(imap_err)?;
        let fetches: Vec<_> = stream.try_collect().await.map_err(imap_err)?;
        for f in fetches {
            if let (Some(uid), Some(body)) = (f.uid, f.body()) {
                bodies.insert(uid, body.to_vec());
            }
        }
    }
    wanted.sort_unstable();
    out.messages = wanted.iter().filter_map(|u| bodies.remove(u)).collect();
    let _ = session.logout().await;
    Ok(out)
}
