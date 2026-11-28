use super::*;
use crate::people::{add, NewPerson};

async fn ada(db: &Db) -> i64 {
    add(
        db,
        &NewPerson {
            full_name: "Ada Lovelace".into(),
            company: Some("Acme".into()),
            title: Some("Engineer".into()),
            how_met: Some("Conference".into()),
            ..NewPerson::default()
        },
        "manual",
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn schedules_lists_and_rejects_bad_input() {
    let db = Db::open_in_memory().await.unwrap();
    let id = ada(&db).await;
    let b = schedule(&db, id, "Coffee", "2026-12-03T17:00:00Z", 45)
        .await
        .unwrap();
    let a = schedule(&db, id, "Intro", "2026-12-01T09:00:00Z", 30)
        .await
        .unwrap();
    let open = open(&db).await.unwrap();
    assert_eq!(open.iter().map(|m| m.id).collect::<Vec<_>>(), vec![a, b]);
    assert_eq!(open[0].person, "Ada Lovelace");
    assert!(schedule(&db, id, " ", "2026-12-01T09:00:00Z", 30)
        .await
        .is_err());
    assert!(schedule(&db, id, "x", "tomorrow", 30).await.is_err());
    assert!(schedule(&db, 999, "x", "2026-12-01T09:00:00Z", 30)
        .await
        .is_err());
}

#[tokio::test]
async fn finishing_writes_the_timeline_and_counts_as_contact() {
    let db = Db::open_in_memory().await.unwrap();
    let id = ada(&db).await;
    let m = schedule(&db, id, "Coffee", "2026-12-03T17:00:00Z", 30)
        .await
        .unwrap();
    finish(&db, m, "Talked about the Acme team").await.unwrap();
    finish(&db, m, "again").await.unwrap(); // second call changes nothing
    let t = people::timeline(&db, id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].summary, "Coffee: Talked about the Acme team");
    assert_eq!(
        people::get(&db, id)
            .await
            .unwrap()
            .last_contacted
            .as_deref(),
        Some("2026-12-03T17:00:00Z")
    );
    assert!(open(&db).await.unwrap().is_empty());
}

#[tokio::test]
async fn prep_note_has_history() {
    let db = Db::open_in_memory().await.unwrap();
    let id = ada(&db).await;
    let first = schedule(&db, id, "Intro", "2026-11-01T09:00:00Z", 30)
        .await
        .unwrap();
    finish(&db, first, "").await.unwrap();
    let next = schedule(&db, id, "Coffee", "2026-12-03T17:00:00Z", 30)
        .await
        .unwrap();
    let note = prep(&db, next).await.unwrap();
    assert!(note.starts_with("Ada Lovelace, Engineer at Acme"));
    assert!(note.contains("How you met: Conference"));
    assert!(note.contains("- 2026-11-01 (meeting): Intro"));
}

#[tokio::test]
async fn calendar_file_has_start_end_and_rolls_over() {
    let db = Db::open_in_memory().await.unwrap();
    let id = ada(&db).await;
    let m = schedule(&db, id, "Dinner, late", "2026-12-31T23:30:00Z", 60)
        .await
        .unwrap();
    let ics = ics(&db, m).await.unwrap();
    assert!(ics.contains("DTSTART:20261231T233000Z"));
    assert!(ics.contains("DTEND:20270101T003000Z"));
    assert!(ics.contains("SUMMARY:Dinner\\, late with Ada Lovelace"));
    assert!(ics.starts_with("BEGIN:VCALENDAR"));
}
