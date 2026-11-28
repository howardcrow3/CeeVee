use std::sync::{Arc, Mutex};

use ceevee_apply::engine::Answers;
use ceevee_bridge::{Backend, Bridge};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

struct Started {
    bridge: Bridge,
    saved: Arc<Mutex<Vec<Vec<String>>>>,
}

async fn start_with(tokens: Vec<String>) -> Started {
    let saved: Arc<Mutex<Vec<Vec<String>>>> = Arc::default();
    let s = saved.clone();
    let backend = Backend {
        answers: Box::new(|| {
            Box::pin(async {
                Ok(Answers::from_parts(
                    "Ada Lovelace",
                    Some("ada@example.com"),
                    None,
                    None,
                    &[],
                    &[],
                ))
            })
        }),
        document: Box::new(|kind| {
            Box::pin(async move {
                Ok((kind == "resume").then(|| ("resume.pdf".to_string(), b"%PDF-1".to_vec())))
            })
        }),
        tokens_changed: Box::new(move |t| s.lock().unwrap().push(t)),
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bridge = Bridge::start_on(listener, backend, tokens).unwrap();
    Started { bridge, saved }
}

async fn start() -> Started {
    start_with(Vec::new()).await
}

/// Send one request and return the status and the JSON body.
async fn call(
    port: u16,
    method: &str,
    path: &str,
    host: Option<&str>,
    extra: &[(&str, &str)],
    body: &str,
) -> (u16, Value) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let host = host.map_or(format!("127.0.0.1:{port}"), str::to_string);
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: {host}\r\n");
    for (k, v) in extra {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    s.write_all(req.as_bytes()).await.unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).await.unwrap();
    let status = out.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = out.split("\r\n\r\n").nth(1).unwrap_or("");
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

const EXT: (&str, &str) = ("Origin", "chrome-extension://abcdef");

/// Pair a browser the way the extension does and return its token.
async fn pair(st: &Started) -> String {
    let port = st.bridge.port();
    let (_, v) = call(
        port,
        "POST",
        "/pair",
        None,
        &[EXT],
        r#"{"browser":"Chrome"}"#,
    )
    .await;
    let id = v["id"].as_str().unwrap().to_string();
    let (_, v) = call(port, "GET", &format!("/pair/{id}"), None, &[EXT], "").await;
    assert_eq!(v["state"], "pending");
    let pending = st.bridge.pending().unwrap();
    assert_eq!(
        (pending.id.as_str(), pending.browser.as_str()),
        (id.as_str(), "Chrome")
    );
    assert!(st.bridge.approve(&id));
    let (_, v) = call(port, "GET", &format!("/pair/{id}"), None, &[EXT], "").await;
    assert_eq!(v["state"], "approved");
    v["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn ping_answers_only_this_computer_and_only_extensions() {
    let st = start().await;
    let port = st.bridge.port();
    let (code, v) = call(port, "GET", "/ping", None, &[], "").await;
    assert_eq!((code, v["app"].as_str()), (200, Some("CeeVee")));
    // A web page's origin is refused, and so is a name that is not this computer.
    let (code, _) = call(
        port,
        "GET",
        "/ping",
        None,
        &[("Origin", "https://evil.example")],
        "",
    )
    .await;
    assert_eq!(code, 403);
    let (code, _) = call(port, "GET", "/ping", Some("evil.example"), &[], "").await;
    assert_eq!(code, 403);
    let (code, _) = call(
        port,
        "GET",
        "/ping",
        Some(&format!("localhost:{port}")),
        &[EXT],
        "",
    )
    .await;
    assert_eq!(code, 200);
    let (code, _) = call(port, "GET", "/nothing", None, &[], "").await;
    assert_eq!(code, 404);
}

#[tokio::test]
async fn nothing_works_without_an_approved_token() {
    let st = start().await;
    let port = st.bridge.port();
    for (m, p) in [("POST", "/plan"), ("GET", "/document/resume")] {
        let (code, _) = call(port, m, p, None, &[EXT], "{}").await;
        assert_eq!(code, 401, "{p}");
        let (code, _) = call(
            port,
            m,
            p,
            None,
            &[EXT, ("Authorization", "Bearer guess")],
            "{}",
        )
        .await;
        assert_eq!(code, 401, "{p}");
    }
}

#[tokio::test]
async fn pairing_gives_a_token_once_and_it_is_remembered() {
    let st = start().await;
    let token = pair(&st).await;
    assert_eq!(token.len(), 64);
    assert_eq!(st.bridge.connected(), 1);
    assert_eq!(
        st.saved.lock().unwrap().last().unwrap(),
        &vec![token.clone()]
    );
    // The token is handed over once.
    let port = st.bridge.port();
    let (code, _) = call(port, "GET", "/pair/ffff", None, &[EXT], "").await;
    assert_eq!(code, 404);
    // And it works, including after a restart that is given the saved token.
    let again = start_with(vec![token.clone()]).await;
    let auth = format!("Bearer {token}");
    let (code, _) = call(
        again.bridge.port(),
        "GET",
        "/document/resume",
        None,
        &[EXT, ("Authorization", &auth)],
        "",
    )
    .await;
    assert_eq!(code, 200);
}

#[tokio::test]
async fn denying_gives_no_token() {
    let st = start().await;
    let port = st.bridge.port();
    let (_, v) = call(port, "POST", "/pair", None, &[EXT], r#"{"browser":"Edge"}"#).await;
    let id = v["id"].as_str().unwrap();
    assert!(st.bridge.deny(id));
    let (_, v) = call(port, "GET", &format!("/pair/{id}"), None, &[EXT], "").await;
    assert_eq!(v["state"], "denied");
    assert!(v["token"].is_null());
    assert_eq!(st.bridge.connected(), 0);
    assert!(!st.bridge.approve(id));
}

#[tokio::test]
async fn a_plan_fills_known_fields_and_flags_the_rest() {
    let st = start().await;
    let token = pair(&st).await;
    let auth = format!("Bearer {token}");
    let body = json!({
        "url": "https://boards.greenhouse.io/acme/jobs/1",
        "page_text": "Apply for this job",
        "fields": [
            {"id": "a", "label": "Email", "kind": "text", "required": true},
            {"id": "b", "label": "Are you authorized to work in the US?", "kind": "text", "required": true},
            {"id": "c", "label": "Favourite colour", "kind": "text", "required": false}
        ]
    })
    .to_string();
    let (code, v) = call(
        st.bridge.port(),
        "POST",
        "/plan",
        None,
        &[EXT, ("Authorization", &auth)],
        &body,
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(
        v["steps"],
        json!([{"field_id": "a", "action": "fill", "value": "ada@example.com"}])
    );
    assert_eq!(v["ready_to_submit"], false);
    let states: Vec<(&str, &str)> = v["checklist"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| (i["label"].as_str().unwrap(), i["state"].as_str().unwrap()))
        .collect();
    assert_eq!(
        states,
        vec![
            ("Are you authorized to work in the US?", "needs_you"),
            ("Favourite colour", "left_blank"),
            ("Email", "filled"),
        ]
    );
}

#[tokio::test]
async fn pages_that_are_never_automated_get_no_answers() {
    let st = start().await;
    let token = pair(&st).await;
    let auth = format!("Bearer {token}");
    let body = json!({
        "url": "https://www.linkedin.com/jobs/view/1/apply/",
        "fields": [{"id": "a", "label": "Email", "kind": "text", "required": true}]
    })
    .to_string();
    let (_, v) = call(
        st.bridge.port(),
        "POST",
        "/plan",
        None,
        &[EXT, ("Authorization", &auth)],
        &body,
    )
    .await;
    assert!(v["refused"].as_str().unwrap().contains("LinkedIn"));
    assert_eq!(v["steps"], json!([]));
}

#[tokio::test]
async fn documents_come_as_base64_and_a_missing_one_is_404() {
    let st = start().await;
    let token = pair(&st).await;
    let auth = format!("Bearer {token}");
    let h = [EXT, ("Authorization", auth.as_str())];
    let (code, v) = call(st.bridge.port(), "GET", "/document/resume", None, &h, "").await;
    assert_eq!(
        (code, v["file_name"].as_str(), v["data_base64"].as_str()),
        (200, Some("resume.pdf"), Some("JVBERi0x"))
    );
    let (code, _) = call(
        st.bridge.port(),
        "GET",
        "/document/cover_letter",
        None,
        &h,
        "",
    )
    .await;
    assert_eq!(code, 404);
}

#[tokio::test]
async fn disconnecting_revokes_every_token_and_big_bodies_are_refused() {
    let st = start().await;
    let token = pair(&st).await;
    let auth = format!("Bearer {token}");
    st.bridge.disconnect_all();
    assert_eq!(st.bridge.connected(), 0);
    assert!(st.saved.lock().unwrap().last().unwrap().is_empty());
    let port = st.bridge.port();
    let (code, _) = call(
        port,
        "GET",
        "/document/resume",
        None,
        &[EXT, ("Authorization", &auth)],
        "",
    )
    .await;
    assert_eq!(code, 401);
    let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let req =
        format!("POST /plan HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: 5000000\r\n\r\n");
    s.write_all(req.as_bytes()).await.unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).await.unwrap();
    assert!(out.starts_with("HTTP/1.1 413"));
}
