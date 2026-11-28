use super::*;
use std::collections::HashMap;

const GH: &str = r#"{"jobs":[
 {"title":"Platform Engineer","absolute_url":"https://boards.greenhouse.io/acme/jobs/1",
  "location":{"name":"Remote, US"},"content":"&lt;p&gt;Build &amp; run the platform&lt;/p&gt;"},
 {"title":"","absolute_url":"https://boards.greenhouse.io/acme/jobs/2"}]}"#;
const LV: &str = r#"[{"text":"Staff Engineer","hostedUrl":"https://jobs.lever.co/globex/9",
 "categories":{"location":"Austin, TX"},"descriptionPlain":"Own the backend."}]"#;

fn gh() -> Board {
    Board {
        kind: BoardKind::Greenhouse,
        token: "acme".into(),
        company: "Acme".into(),
    }
}
fn lv() -> Board {
    Board {
        kind: BoardKind::Lever,
        token: "globex".into(),
        company: "Globex".into(),
    }
}

struct Fake(HashMap<String, std::result::Result<String, String>>);
impl Fetcher for Fake {
    fn get(&self, url: String) -> FetchFuture {
        let r = self
            .0
            .get(&url)
            .cloned()
            .unwrap_or(Err("no such board".into()));
        Box::pin(async move { r.map_err(Error::Other) })
    }
}

#[test]
fn greenhouse_feed_parses_and_skips_untitled() {
    let v = parse_feed(&gh(), GH).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].location.as_deref(), Some("Remote, US"));
    assert_eq!(v[0].body.as_deref(), Some("Build & run the platform"));
    assert_eq!(v[0].source, "greenhouse");
}

#[test]
fn lever_feed_parses() {
    let v = parse_feed(&lv(), LV).unwrap();
    assert_eq!(v[0].title, "Staff Engineer");
    assert_eq!(v[0].location.as_deref(), Some("Austin, TX"));
}

#[test]
fn wrong_shape_is_an_error() {
    assert!(parse_feed(&gh(), "[]").is_err());
    assert!(parse_feed(&lv(), "{}").is_err());
    assert!(parse_feed(&lv(), "not json").is_err());
}

#[test]
fn feed_urls() {
    assert_eq!(
        gh().feed_url(),
        "https://boards-api.greenhouse.io/v1/boards/acme/jobs?content=true"
    );
    assert_eq!(
        lv().feed_url(),
        "https://api.lever.co/v0/postings/globex?mode=json"
    );
}

#[tokio::test]
async fn run_stores_once_and_survives_a_bad_board() {
    let db = Db::open_in_memory().await.unwrap();
    let mut m = HashMap::new();
    m.insert(gh().feed_url(), Ok(GH.to_string()));
    m.insert(lv().feed_url(), Err("answered 404".to_string()));
    let f = Fake(m);
    let boards = [gh(), lv()];
    let a = run(&db, &f, &boards, "2026-11-01T10:00:00Z").await.unwrap();
    assert_eq!((a.boards_ok, a.new, a.duplicates), (1, 1, 0));
    assert_eq!(a.failures.len(), 1);
    assert!(a.failures[0].starts_with("Globex"));
    let b = run(&db, &f, &boards, "2026-11-02T10:00:00Z").await.unwrap();
    assert_eq!((b.new, b.duplicates), (0, 1));
    assert_eq!(postings::recent(&db, 10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn boards_round_trip_and_reject_bad_tokens() {
    let db = Db::open_in_memory().await.unwrap();
    assert!(load_boards(&db).await.unwrap().is_empty());
    save_boards(&db, &[gh(), lv()]).await.unwrap();
    assert_eq!(load_boards(&db).await.unwrap(), vec![gh(), lv()]);
    let bad = Board {
        token: "a/../b".into(),
        ..gh()
    };
    assert!(save_boards(&db, &[bad]).await.is_err());
    let blank = Board {
        company: " ".into(),
        ..gh()
    };
    assert!(save_boards(&db, &[blank]).await.is_err());
}

#[test]
fn iso_dates() {
    assert_eq!(iso_from_secs(0), "1970-01-01T00:00:00Z");
    assert_eq!(iso_from_secs(1_784_000_000), "2026-07-14T03:33:20Z");
    assert_eq!(iso_from_secs(951_782_400), "2000-02-29T00:00:00Z");
}

async fn serve_once(body: Vec<u8>) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/feed", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = s.read(&mut buf).await;
        let head = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        let _ = s.write_all(head.as_bytes()).await;
        let _ = s.write_all(&body).await;
    });
    url
}

#[tokio::test]
async fn a_feed_within_the_limit_is_read_and_one_over_it_is_refused() {
    let url = serve_once(b"{\"jobs\":[]}".to_vec()).await;
    let res = reqwest::get(&url).await.unwrap();
    assert_eq!(read_capped(res, 1024).await.unwrap(), "{\"jobs\":[]}");

    let url = serve_once(vec![b'x'; 2048]).await;
    let res = reqwest::get(&url).await.unwrap();
    let err = read_capped(res, 1024).await.unwrap_err();
    assert!(err.contains("too large"), "{err}");
}
