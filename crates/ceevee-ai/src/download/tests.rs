use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use super::*;
use crate::catalog::{ModelEntry, Role};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Minimal HTTP/1.1 file server on localhost. Counts requests and records Range headers.
struct Server {
    url: String,
    ranges: Arc<std::sync::Mutex<Vec<Option<u64>>>>,
}

async fn serve(body: Vec<u8>, honor_range: bool) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/m.gguf", listener.local_addr().unwrap());
    let ranges = Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = ranges.clone();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let offset = req
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("range: bytes=")
                        .map(str::to_owned)
                })
                .and_then(|r| r.trim_end_matches('-').trim().parse::<u64>().ok());
            seen.lock().unwrap().push(offset);
            let len = body.len() as u64;
            let resp: Vec<u8> = match offset {
                Some(o) if honor_range && o >= len => {
                    format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{len}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").into_bytes()
                }
                Some(o) if honor_range => {
                    let part = &body[o as usize..];
                    let mut r = format!("HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {o}-{}/{len}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", len - 1, part.len()).into_bytes();
                    r.extend_from_slice(part);
                    r
                }
                _ => {
                    let mut r = format!("HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n").into_bytes();
                    r.extend_from_slice(&body);
                    r
                }
            };
            let _ = sock.write_all(&resp).await;
            let _ = sock.shutdown().await;
        }
    });
    Server { url, ranges }
}

fn payload() -> Vec<u8> {
    (0..200_000u32).map(|i| (i % 251) as u8).collect()
}

fn entry(url: &str, body: &[u8], sha: Option<String>) -> ModelEntry {
    ModelEntry {
        id: "t".into(),
        role: Role::General,
        name: "t".into(),
        file: "m.gguf".into(),
        size_bytes: body.len() as u64,
        license: "MIT".into(),
        min_ram_gb: 1,
        default: true,
        url: Some(url.into()),
        sha256: sha.or_else(|| Some(hex(&Sha256::digest(body)))),
    }
}

#[tokio::test]
async fn downloads_and_verifies() {
    let body = payload();
    let srv = serve(body.clone(), true).await;
    let dir = tempfile::tempdir().unwrap();
    let mut last = None;
    let path = download(
        &entry(&srv.url, &body, None),
        dir.path(),
        &AtomicBool::new(false),
        |p| last = Some(p),
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), body);
    assert_eq!(last.unwrap().downloaded, body.len() as u64);
    assert!(!dir.path().join("m.gguf.partial").exists());
}

#[tokio::test]
async fn resumes_from_a_partial_file_with_a_range_request() {
    let body = payload();
    let srv = serve(body.clone(), true).await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("m.gguf.partial"), &body[..70_000]).unwrap();
    let path = download(
        &entry(&srv.url, &body, None),
        dir.path(),
        &AtomicBool::new(false),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), body);
    assert_eq!(*srv.ranges.lock().unwrap(), vec![Some(70_000)]);
}

#[tokio::test]
async fn restarts_when_the_server_ignores_range() {
    let body = payload();
    let srv = serve(body.clone(), false).await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("m.gguf.partial"), &body[..70_000]).unwrap();
    let path = download(
        &entry(&srv.url, &body, None),
        dir.path(),
        &AtomicBool::new(false),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), body);
}

#[tokio::test]
async fn a_complete_partial_is_accepted_after_416() {
    let body = payload();
    let srv = serve(body.clone(), true).await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("m.gguf.partial"), &body).unwrap();
    let path = download(
        &entry(&srv.url, &body, None),
        dir.path(),
        &AtomicBool::new(false),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), body);
}

#[tokio::test]
async fn corrupted_partial_is_replaced() {
    let body = payload();
    let srv = serve(body.clone(), true).await;
    let dir = tempfile::tempdir().unwrap();
    let mut bad = body[..70_000].to_vec();
    bad[10] ^= 0xff;
    std::fs::write(dir.path().join("m.gguf.partial"), bad).unwrap();
    let path = download(
        &entry(&srv.url, &body, None),
        dir.path(),
        &AtomicBool::new(false),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), body);
    assert_eq!(*srv.ranges.lock().unwrap(), vec![Some(70_000), None]);
}

#[tokio::test]
async fn wrong_checksum_discards_the_file() {
    let body = payload();
    let srv = serve(body.clone(), true).await;
    let dir = tempfile::tempdir().unwrap();
    let wrong = Some("0".repeat(64));
    let err = download(
        &entry(&srv.url, &body, wrong),
        dir.path(),
        &AtomicBool::new(false),
        |_| {},
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("checksum"), "{err}");
    assert!(!dir.path().join("m.gguf").exists());
    assert!(!dir.path().join("m.gguf.partial").exists());
}

#[tokio::test]
async fn cancel_keeps_the_partial_and_a_verified_file_is_not_refetched() {
    let body = payload();
    let srv = serve(body.clone(), true).await;
    let dir = tempfile::tempdir().unwrap();
    let e = entry(&srv.url, &body, None);
    let err = download(&e, dir.path(), &AtomicBool::new(true), |_| {})
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Cancelled));
    assert!(dir.path().join("m.gguf.partial").exists());

    download(&e, dir.path(), &AtomicBool::new(false), |_| {})
        .await
        .unwrap();
    let before = srv.ranges.lock().unwrap().len();
    download(&e, dir.path(), &AtomicBool::new(false), |_| {})
        .await
        .unwrap();
    assert_eq!(
        srv.ranges.lock().unwrap().len(),
        before,
        "no request for an intact file"
    );
}

#[tokio::test]
async fn unpublished_models_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = entry("http://x", b"", None);
    e.url = None;
    let err = download(&e, dir.path(), &AtomicBool::new(false), |_| {})
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not published"));
}
