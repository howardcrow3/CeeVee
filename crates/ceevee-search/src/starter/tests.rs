use super::*;
use crate::categories::{self, Criteria};
use std::collections::{HashMap, HashSet};

struct Fake(HashMap<String, std::result::Result<String, String>>);
impl Fetcher for Fake {
    fn get(&self, url: String) -> sources::FetchFuture {
        let r = self
            .0
            .get(&url)
            .cloned()
            .unwrap_or_else(|| Err("not found".into()));
        Box::pin(async move { r.map_err(ceevee_core::Error::Other) })
    }
}

fn gh_feed(url_id: u32, title: &str) -> String {
    format!(
        r#"{{"jobs":[{{"title":"{title}","absolute_url":"https://boards.greenhouse.io/x/jobs/{url_id}",
        "location":{{"name":"Remote, US"}},"content":"Own the roadmap"}}]}}"#
    )
}

fn feed_url(token: &str) -> String {
    Board {
        kind: BoardKind::Greenhouse,
        token: token.into(),
        company: token.into(),
    }
    .feed_url()
}

#[test]
fn the_list_is_clean() {
    let all = boards();
    assert!(all.len() >= 20);
    let mut seen = HashSet::new();
    for b in &all {
        assert!(
            b.token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "{}",
            b.token
        );
        assert!(!b.company.is_empty());
        assert!(
            seen.insert(format!("{:?}:{}", b.kind, b.token)),
            "duplicate {}",
            b.token
        );
    }
}

#[tokio::test]
async fn first_search_finds_jobs_with_no_boards_added() {
    let db = Db::open_in_memory().await.unwrap();
    categories::create(
        &db,
        "Product",
        &Criteria {
            titles: vec!["product manager".into()],
            ..Criteria::default()
        },
    )
    .await
    .unwrap();
    let mut m = HashMap::new();
    m.insert(feed_url("stripe"), Ok(gh_feed(1, "Product Manager")));
    let s = run_all(&db, &Fake(m), "2026-11-26T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(s.new, 1);
    assert!(s.boards_ok >= 1);
    // A dead starter board is not shown to the person as a failure.
    assert!(s.failures.is_empty());
}

#[tokio::test]
async fn own_board_failures_are_still_reported() {
    let db = Db::open_in_memory().await.unwrap();
    sources::save_boards(
        &db,
        &[Board {
            kind: BoardKind::Greenhouse,
            token: "mine".into(),
            company: "Mine".into(),
        }],
    )
    .await
    .unwrap();
    let s = run_all(&db, &Fake(HashMap::new()), "2026-11-26T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(s.failures.len(), 1);
    assert!(s.failures[0].starts_with("Mine"));
}

#[tokio::test]
async fn can_be_turned_off_and_on() {
    let db = Db::open_in_memory().await.unwrap();
    assert!(is_enabled(&db).await.unwrap());
    set_enabled(&db, false).await.unwrap();
    assert!(!is_enabled(&db).await.unwrap());
    let mut m = HashMap::new();
    m.insert(feed_url("stripe"), Ok(gh_feed(1, "Product Manager")));
    let s = run_all(&db, &Fake(m), "2026-11-26T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(s.new, 0);
    set_enabled(&db, true).await.unwrap();
    assert!(is_enabled(&db).await.unwrap());
}

#[tokio::test]
async fn a_board_the_person_added_is_not_searched_twice() {
    let db = Db::open_in_memory().await.unwrap();
    sources::save_boards(
        &db,
        &[Board {
            kind: BoardKind::Greenhouse,
            token: "Stripe".into(),
            company: "Stripe Inc".into(),
        }],
    )
    .await
    .unwrap();
    let mut m = HashMap::new();
    m.insert(feed_url("Stripe"), Ok(gh_feed(1, "Product Manager")));
    let s = run_all(&db, &Fake(m), "2026-11-26T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(s.new, 1);
    assert_eq!(s.duplicates, 0);
}
