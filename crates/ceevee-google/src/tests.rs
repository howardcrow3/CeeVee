use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::gmail::{Gmail, MailApi};
use crate::oauth::{decode, encode, pkce, query_param, OAuthClient, GMAIL_READONLY};

/// A one-route-at-a-time HTTP server: `routes` maps a path prefix to (status, body). Returns the
/// base URL and the log of request lines and bodies.
async fn http(routes: Vec<(&'static str, u16, String)>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let log = Arc::new(Mutex::new(vec![]));
    let log2 = log.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let routes = routes.clone();
            let log = log2.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 16384];
                let mut n = 0;
                loop {
                    let k = sock.read(&mut buf[n..]).await.unwrap_or(0);
                    if k == 0 {
                        break;
                    }
                    n += k;
                    let text = String::from_utf8_lossy(&buf[..n]).into_owned();
                    if let Some(h) = text.find("\r\n\r\n") {
                        let len = text[..h]
                            .to_ascii_lowercase()
                            .split("content-length:")
                            .nth(1)
                            .and_then(|r| r.split("\r\n").next())
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if n >= h + 4 + len {
                            break;
                        }
                    }
                }
                let text = String::from_utf8_lossy(&buf[..n]).into_owned();
                let line = text.lines().next().unwrap_or("").to_string();
                let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
                log.lock().unwrap().push(format!("{line} || {body}"));
                let path = line.split_whitespace().nth(1).unwrap_or("");
                let (status, out) = routes
                    .iter()
                    .find(|r| path.starts_with(r.0))
                    .map_or((404, "{}".to_string()), |r| (r.1, r.2.clone()));
                let resp = format!("HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{out}", out.len());
                let _ = sock.write_all(resp.as_bytes()).await;
            });
        }
    });
    (base, log)
}

#[test]
fn pkce_matches_the_rfc_example() {
    use sha2::{Digest, Sha256};
    let v = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    assert_eq!(
        URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes())),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    let p = pkce().unwrap();
    assert!(p.verifier.len() >= 43);
    assert_ne!(p.verifier, pkce().unwrap().verifier);
}

#[test]
fn escaping_round_trips() {
    let s = "a b&c=d/é?";
    assert_eq!(decode(&encode(s)), s);
    assert_eq!(
        query_param("code=4%2F0A&state=x", "code").as_deref(),
        Some("4/0A")
    );
    assert_eq!(decode("100%"), "100%");
    assert_eq!(decode("%zz"), "%zz");
}

#[test]
fn the_sign_in_address_asks_only_for_what_it_needs() {
    let c = OAuthClient::google("id.apps.googleusercontent.com", None);
    let u = c.authorization_url(
        "http://127.0.0.1:5000",
        "CH",
        "ST",
        &["openid", "email", GMAIL_READONLY],
    );
    assert!(u.contains("code_challenge=CH") && u.contains("code_challenge_method=S256"));
    assert!(
        u.contains("gmail.readonly") && !u.contains("gmail.send") && !u.contains("mail.google.com")
    );
    assert!(u.contains("access_type=offline"));
}

fn id_token(email: &str) -> String {
    let payload = URL_SAFE_NO_PAD.encode(format!("{{\"email\":\"{email}\"}}"));
    format!("h.{payload}.s")
}

#[tokio::test]
async fn the_whole_sign_in_flow_gets_tokens_and_the_address() {
    let reply = format!("{{\"access_token\":\"AT\",\"refresh_token\":\"RT\",\"expires_in\":3600,\"scope\":\"openid email {GMAIL_READONLY}\",\"id_token\":\"{}\"}}", id_token("Me@Gmail.com"));
    let (base, log) = http(vec![("/token", 200, reply)]).await;
    let mut c = OAuthClient::google("cid", Some("secret"));
    c.token_url = format!("{base}/token");
    let t = c
        .sign_in(&["openid", "email", GMAIL_READONLY], |url| {
            let url = url.to_string();
            tokio::spawn(async move {
                let redirect =
                    decode(&query_param(url.split_once('?').unwrap().1, "redirect_uri").unwrap());
                let state = query_param(url.split_once('?').unwrap().1, "state").unwrap();
                // A stray request with the wrong state is ignored.
                let _ = reqwest::get(format!("{redirect}/?code=evil&state=nope")).await;
                let page = reqwest::get(format!("{redirect}/?code=GOOD%2F1&state={state}"))
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();
                assert!(page.contains("Google is connected"), "{page}");
            });
        })
        .await
        .unwrap();
    assert_eq!(t.access_token, "AT");
    assert_eq!(t.refresh_token.as_deref(), Some("RT"));
    assert_eq!(t.email.as_deref(), Some("me@gmail.com"));
    assert!(t.scopes.iter().any(|s| s == GMAIL_READONLY));
    let sent = log.lock().unwrap().join("\n");
    assert!(
        sent.contains("code=GOOD%2F1")
            && sent.contains("code_verifier=")
            && sent.contains("client_secret=secret")
    );
    assert!(!sent.contains("evil"));
}

#[tokio::test]
async fn refusing_access_is_reported_plainly() {
    let (base, _) = http(vec![]).await;
    let mut c = OAuthClient::google("cid", None);
    c.token_url = format!("{base}/token");
    let e = c
        .sign_in(&["openid"], |url| {
            let url = url.to_string();
            tokio::spawn(async move {
                let q = url.split_once('?').unwrap().1.to_string();
                let redirect = decode(&query_param(&q, "redirect_uri").unwrap());
                let state = query_param(&q, "state").unwrap();
                let page = reqwest::get(format!("{redirect}/?error=access_denied&state={state}"))
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();
                assert!(page.contains("Access was not granted"), "{page}");
            });
        })
        .await
        .unwrap_err();
    assert!(
        e.to_string().contains("you chose not to allow access"),
        "{e}"
    );
}

#[tokio::test]
async fn refreshing_returns_a_new_access_token() {
    let (base, log) = http(vec![(
        "/token",
        200,
        "{\"access_token\":\"NEW\",\"expires_in\":3599}".into(),
    )])
    .await;
    let mut c = OAuthClient::google("cid", None);
    c.token_url = format!("{base}/token");
    assert_eq!(c.refresh("RT").await.unwrap(), "NEW");
    assert!(log.lock().unwrap()[0].contains("grant_type=refresh_token"));
    let (base, _) = http(vec![("/token", 400, "{\"error\":\"invalid_grant\",\"error_description\":\"Token has been expired or revoked.\"}".into())]).await;
    c.token_url = format!("{base}/token");
    assert!(c
        .refresh("RT")
        .await
        .unwrap_err()
        .to_string()
        .contains("expired or revoked"));
}

#[tokio::test]
async fn gmail_lists_pages_reads_headers_and_raw_messages() {
    let raw = "From: a@b.c\r\nSubject: Hi\r\n\r\nbody";
    let (base, log) = http(vec![
        ("/gmail/v1/users/me/profile", 200, "{\"emailAddress\":\"Me@Gmail.com\"}".into()),
        ("/gmail/v1/users/me/messages?maxResults=2&q=in%3Ainbox", 200, "{\"messages\":[{\"id\":\"m1\"},{\"id\":\"m2\"}],\"nextPageToken\":\"P2\"}".into()),
        ("/gmail/v1/users/me/messages/m1?format=raw", 200, format!("{{\"raw\":\"{}\"}}", URL_SAFE_NO_PAD.encode(raw))),
        ("/gmail/v1/users/me/messages/m1?format=metadata", 200, "{\"payload\":{\"headers\":[{\"name\":\"From\",\"value\":\"Dana <d@acme.com>\"},{\"name\":\"Subject\",\"value\":\"Hi\\nthere\"}]}}".into()),
    ])
    .await;
    let g = Gmail::with_base("TOKEN", &base);
    assert_eq!(g.address().await.unwrap(), "me@gmail.com");
    assert_eq!(g.list_ids("in:inbox", 2).await.unwrap(), vec!["m1", "m2"]);
    assert_eq!(g.raw("m1").await.unwrap(), raw.as_bytes());
    let h = String::from_utf8(g.headers("m1").await.unwrap()).unwrap();
    assert!(
        h.contains("From: Dana <d@acme.com>\r\n")
            && h.contains("Subject: Hi there\r\n")
            && h.ends_with("\r\n\r\n")
    );
    assert!(log
        .lock()
        .unwrap()
        .iter()
        .any(|l| l.contains("metadataHeaders=From")));
}

#[tokio::test]
async fn a_revoked_grant_says_what_to_do() {
    let (base, _) = http(vec![("/gmail", 401, "{}".into())]).await;
    let e = Gmail::with_base("T", &base).address().await.unwrap_err();
    assert!(e.to_string().contains("Connect Google again"), "{e}");
}

#[tokio::test]
async fn closing_the_browser_does_not_trap_the_app_in_waiting() {
    use std::time::Duration;
    let c = OAuthClient::google("cid", None);
    // Nobody ever comes back: the wait ends by itself.
    let cancel = tokio::sync::Notify::new();
    let e = c
        .sign_in_cancellable(&["openid"], |_| {}, &cancel, Duration::from_millis(150))
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("timed out") && e.contains("try again"), "{e}");

    // The person presses Cancel: it ends at once, long before the timeout.
    let cancel = Arc::new(tokio::sync::Notify::new());
    let c2 = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        c2.notify_one();
    });
    let started = std::time::Instant::now();
    let e = c
        .sign_in_cancellable(&["openid"], |_| {}, &cancel, Duration::from_secs(60))
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("cancelled"), "{e}");
    assert!(started.elapsed() < Duration::from_secs(5));

    // A cancel pressed just before the wait starts is not lost.
    cancel.notify_one();
    assert!(c
        .sign_in_cancellable(&["openid"], |_| {}, &cancel, Duration::from_secs(60))
        .await
        .is_err());

    // After all that, a fresh sign-in still completes.
    let reply = format!("{{\"access_token\":\"AT\",\"refresh_token\":\"RT\",\"expires_in\":3600,\"scope\":\"openid {GMAIL_READONLY}\",\"id_token\":\"{}\"}}", id_token("me@gmail.com"));
    let (base, _) = http(vec![("/token", 200, reply)]).await;
    let mut c = OAuthClient::google("cid", None);
    c.token_url = format!("{base}/token");
    let t = c
        .sign_in_cancellable(
            &["openid"],
            |url| {
                let url = url.to_string();
                tokio::spawn(async move {
                    let q = url.split_once('?').unwrap().1.to_string();
                    let redirect = decode(&query_param(&q, "redirect_uri").unwrap());
                    let state = query_param(&q, "state").unwrap();
                    let _ = reqwest::get(format!("{redirect}/?code=C&state={state}")).await;
                });
            },
            &tokio::sync::Notify::new(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    assert_eq!(t.email.as_deref(), Some("me@gmail.com"));
}

#[tokio::test]
async fn signing_out_revokes_the_token_with_google() {
    let (base, log) = http(vec![("/revoke", 200, "{}".into())]).await;
    let mut c = OAuthClient::google("cid", None);
    c.revoke_url = format!("{base}/revoke");
    c.revoke("rt/1").await.unwrap();
    assert!(
        log.lock().unwrap().join("\n").contains("POST /revoke")
            && log.lock().unwrap().join("\n").contains("token=rt%2F1")
    );

    // A token Google already forgot is fine; any other failure is reported.
    let (base, _) = http(vec![(
        "/revoke",
        400,
        "{\"error\":\"invalid_token\"}".into(),
    )])
    .await;
    c.revoke_url = format!("{base}/revoke");
    c.revoke("old").await.unwrap();
    let (base, _) = http(vec![("/revoke", 500, "{}".into())]).await;
    c.revoke_url = format!("{base}/revoke");
    assert!(c.revoke("x").await.is_err());
}

#[test]
fn result_pages_are_self_contained() {
    for page in [
        include_str!("pages/oauth_done.html"),
        include_str!("pages/oauth_denied.html"),
        include_str!("pages/oauth_error.html"),
    ] {
        assert!(!page.contains("http://") && !page.contains("https://"));
        assert!(!page.contains("<script") && !page.contains("<link"));
    }
}

#[tokio::test]
async fn the_browser_only_says_connected_after_the_app_has_saved_the_sign_in() {
    let reply = format!("{{\"access_token\":\"AT\",\"refresh_token\":\"RT\",\"expires_in\":3600,\"scope\":\"openid email {GMAIL_READONLY}\",\"id_token\":\"{}\"}}", id_token("me@gmail.com"));
    let (base, _) = http(vec![("/token", 200, reply)]).await;
    let mut c = OAuthClient::google("cid", None);
    c.token_url = format!("{base}/token");
    let page = Arc::new(Mutex::new(String::new()));
    let page2 = page.clone();
    let e = c
        .sign_in_then(
            &["openid"],
            move |url| {
                let url = url.to_string();
                tokio::spawn(async move {
                    let q = url.split_once('?').unwrap().1.to_string();
                    let redirect = decode(&query_param(&q, "redirect_uri").unwrap());
                    let state = query_param(&q, "state").unwrap();
                    let text = reqwest::get(format!("{redirect}/?code=C&state={state}"))
                        .await
                        .unwrap()
                        .text()
                        .await
                        .unwrap();
                    *page2.lock().unwrap() = text;
                });
            },
            &tokio::sync::Notify::new(),
            std::time::Duration::from_secs(10),
            |_tokens| async { Err::<(), String>("could not save to the keychain".into()) },
        )
        .await
        .unwrap_err();
    assert!(
        e.to_string().contains("could not save to the keychain"),
        "{e}"
    );
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let page = page.lock().unwrap().clone();
    assert!(!page.contains("Google is connected"), "{page}");
}
