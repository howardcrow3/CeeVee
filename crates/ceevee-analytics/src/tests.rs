use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn ctx() -> Context {
    Context {
        version: "1.2.3".into(),
        os: "windows x86_64".into(),
    }
}

/// A one-shot local server that records the first request it gets.
async fn capture() -> (String, tokio::task::JoinHandle<String>) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mp/collect", l.local_addr().unwrap());
    let h = tokio::spawn(async move {
        let (mut s, _) = l.accept().await.unwrap();
        let mut buf = vec![0u8; 8192];
        let n = s.read(&mut buf).await.unwrap();
        let _ = s
            .write_all(b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\n\r\n")
            .await;
        String::from_utf8_lossy(&buf[..n]).to_string()
    });
    (url, h)
}

async fn make(target: Option<Target>) -> Analytics {
    Analytics::new(
        ceevee_db::Db::open_in_memory().await.unwrap(),
        target,
        ctx(),
    )
}

fn target(url: &str) -> Target {
    let mut t = Target::new("G-TEST", "secret");
    t.endpoint = url.into();
    t
}

#[tokio::test]
async fn default_is_unset_and_sends_nothing() {
    let (url, h) = capture().await;
    let a = make(Some(target(&url))).await;
    assert_eq!(a.consent().await, Consent::Unset);
    a.track(Event::AppStart).await;
    assert!(!h.is_finished(), "no request may be made without consent");
    h.abort();
}

#[tokio::test]
async fn denied_sends_nothing() {
    let (url, h) = capture().await;
    let a = make(Some(target(&url))).await;
    a.set_consent(Consent::Denied).await.unwrap();
    a.track(Event::AppStart).await;
    assert!(!h.is_finished());
    h.abort();
}

#[tokio::test]
async fn no_credentials_sends_nothing() {
    let a = make(None).await;
    a.set_consent(Consent::Granted).await.unwrap();
    a.track(Event::AppStart).await; // must simply return
}

#[tokio::test]
async fn granted_sends_only_allowlisted_fields() {
    let (url, h) = capture().await;
    let a = make(Some(target(&url))).await;
    a.set_consent(Consent::Granted).await.unwrap();
    a.track(Event::FeatureUsed(Feature::JobSearchRun)).await;
    let req = h.await.unwrap();
    assert!(req.starts_with("POST /mp/collect?"));
    assert!(req.contains("measurement_id=G-TEST") && req.contains("api_secret=secret"));
    let body: Value = serde_json::from_str(req.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    let ev = &body["events"][0];
    assert_eq!(ev["name"], "feature_used");
    let mut keys: Vec<_> = ev["params"].as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        ["app_version", "engagement_time_msec", "feature", "os"]
    );
    assert_eq!(ev["params"]["feature"], "job_search_run");
    let mut top: Vec<_> = body.as_object().unwrap().keys().cloned().collect();
    top.sort();
    assert_eq!(top, ["client_id", "events"]);
}

#[tokio::test]
async fn opting_out_forgets_the_id() {
    let a = make(Some(Target::new("G-X", "s"))).await;
    a.set_consent(Consent::Granted).await.unwrap();
    let first = a.client_id().await.unwrap();
    assert_eq!(
        a.client_id().await.unwrap(),
        first,
        "id is stable while opted in"
    );
    a.set_consent(Consent::Denied).await.unwrap();
    a.set_consent(Consent::Granted).await.unwrap();
    assert_ne!(
        a.client_id().await.unwrap(),
        first,
        "a new opt-in gets a new id"
    );
}

#[tokio::test]
async fn unreachable_server_never_fails_the_caller() {
    let a = make(Some(target("http://127.0.0.1:1/mp/collect"))).await;
    a.set_consent(Consent::Granted).await.unwrap();
    a.track(Event::Crash).await;
}
