use super::*;
use crate::postings::{ingest, Ingest, NewPosting};
use serde_json::json;

async fn posting(db: &Db) -> i64 {
    let p = NewPosting {
        source: "greenhouse".into(),
        url: "https://boards.greenhouse.io/acme/jobs/7".into(),
        company: "Acme".into(),
        title: "Analyst".into(),
        location: None,
        level: None,
        pay: None,
        body: Some("Own the reporting.".into()),
    };
    match ingest(db, &p, "2026-11-26T00:00:00Z").await.unwrap() {
        Ingest::New(id) => id,
        other => panic!("{other:?}"),
    }
}

fn new_record(posting_id: i64, event: Event) -> NewRecord {
    NewRecord {
        posting_id,
        application_id: None,
        event,
        url: "https://boards.greenhouse.io/acme/jobs/7".into(),
        ats: Some("greenhouse".into()),
        resume_version_id: None,
        cover_letter_version_id: None,
        filled: json!([{"label": "Email", "value": "ada@example.com", "review": false}]),
        skipped: json!([{"label": "Why us?", "required": true, "reason": "no saved answer"}]),
    }
}

#[tokio::test]
async fn saves_and_reads_back_with_the_posting_text() {
    let db = Db::open_in_memory().await.unwrap();
    let pid = posting(&db).await;
    save(&db, &new_record(pid, Event::Prepared)).await.unwrap();
    let got = list_for_posting(&db, pid).await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].event, "prepared");
    assert_eq!(got[0].job_title.as_deref(), Some("Analyst"));
    assert_eq!(got[0].posting_text.as_deref(), Some("Own the reporting."));
    assert_eq!(got[0].filled[0]["label"], "Email");
    assert_eq!(got[0].skipped[0]["required"], true);
}

#[tokio::test]
async fn keeps_the_posting_text_when_the_posting_changes_later() {
    let db = Db::open_in_memory().await.unwrap();
    let pid = posting(&db).await;
    save(&db, &new_record(pid, Event::Prepared)).await.unwrap();
    sqlx::query("UPDATE posting SET body = 'Role closed.' WHERE id = ?")
        .bind(pid)
        .execute(db.pool())
        .await
        .unwrap();
    let got = list_for_posting(&db, pid).await.unwrap();
    assert_eq!(got[0].posting_text.as_deref(), Some("Own the reporting."));
}

#[tokio::test]
async fn lists_newest_first() {
    let db = Db::open_in_memory().await.unwrap();
    let pid = posting(&db).await;
    save(&db, &new_record(pid, Event::Prepared)).await.unwrap();
    save(&db, &new_record(pid, Event::Submitted)).await.unwrap();
    let got = list_for_posting(&db, pid).await.unwrap();
    assert_eq!(got[0].event, "submitted");
    assert_eq!(got[1].event, "prepared");
}

#[tokio::test]
async fn rejects_a_missing_posting_and_bad_lists() {
    let db = Db::open_in_memory().await.unwrap();
    assert!(save(&db, &new_record(99, Event::Prepared)).await.is_err());
    let pid = posting(&db).await;
    let mut r = new_record(pid, Event::Prepared);
    r.filled = json!({"not": "a list"});
    assert!(save(&db, &r).await.is_err());
}
