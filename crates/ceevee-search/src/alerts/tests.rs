use super::*;
use crate::categories::{self, Criteria};
use crate::postings::{self, NewPosting};

const T0: &str = "2026-11-01T10:00:00Z";
const T1: &str = "2026-11-02T10:00:00Z";

async fn add(db: &Db, url: &str, title: &str, at: &str) -> i64 {
    let p = NewPosting {
        source: "x".into(),
        url: url.into(),
        company: "Acme".into(),
        title: title.into(),
        ..NewPosting::default()
    };
    let postings::Ingest::New(id) = postings::ingest(db, &p, at).await.unwrap() else {
        panic!("duplicate")
    };
    categories::score_posting(db, id).await.unwrap();
    id
}

async fn pm_category(db: &Db) {
    let c = Criteria {
        titles: vec!["product manager".into()],
        ..Criteria::default()
    };
    categories::create(db, "PM", &c).await.unwrap();
}

#[tokio::test]
async fn only_new_good_matches_since_the_cutoff_count() {
    let db = Db::open_in_memory().await.unwrap();
    pm_category(&db).await;
    add(&db, "https://a/old", "Product Manager", T0).await;
    add(&db, "https://a/good", "Senior Product Manager", T1).await;
    add(&db, "https://a/poor", "Plumber", T1).await;
    let dismissed = add(&db, "https://a/gone", "Product Manager II", T1).await;
    sqlx::query("UPDATE posting SET state = 'dismissed' WHERE id = ?")
        .bind(dismissed)
        .execute(db.pool())
        .await
        .unwrap();

    let found = new_matches(&db, T1, min_score(&db).await).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "Senior Product Manager");
    assert_eq!(count_since(&db, T1).await.unwrap(), 2); // good + poor; dismissed excluded
}

#[tokio::test]
async fn min_score_setting_overrides_the_default() {
    let db = Db::open_in_memory().await.unwrap();
    assert_eq!(min_score(&db).await, DEFAULT_MIN_SCORE);
    db.set_setting("notify.min_score", "85").await.unwrap();
    assert_eq!(min_score(&db).await, 85.0);
    db.set_setting("notify.min_score", "junk").await.unwrap();
    assert_eq!(min_score(&db).await, DEFAULT_MIN_SCORE);
}

fn m(title: &str, company: Option<&str>) -> Match {
    Match {
        title: title.into(),
        company: company.map(Into::into),
        score: 90.0,
    }
}

#[test]
fn message_wording() {
    assert_eq!(message(&[]), None);
    let (t, b) = message(&[m("PM", Some("Acme"))]).unwrap();
    assert_eq!(
        (t.as_str(), b.as_str()),
        ("1 new job matches your search", "PM at Acme")
    );
    let five: Vec<_> = (1..=5).map(|i| m(&format!("Job {i}"), None)).collect();
    let (t, b) = message(&five).unwrap();
    assert_eq!(t, "5 new jobs match your search");
    assert_eq!(b, "Job 1; Job 2; Job 3; and 2 more");
}
