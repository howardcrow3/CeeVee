use super::*;
use crate::intros::{record_ask, set_outcome};
use crate::people::{add, add_touch, Kind, NewPerson};

const NOW: &str = "2026-12-10T12:00:00Z";

async fn person(db: &Db, name: &str) -> i64 {
    add(
        db,
        &NewPerson {
            full_name: name.into(),
            ..NewPerson::default()
        },
        "manual",
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn goal_defaults_to_three_and_validates() {
    let db = Db::open_in_memory().await.unwrap();
    assert_eq!(weekly_goal(&db).await.unwrap(), 3);
    set_weekly_goal(&db, 5).await.unwrap();
    assert_eq!(weekly_goal(&db).await.unwrap(), 5);
    assert!(set_weekly_goal(&db, 0).await.is_err());
    assert!(set_weekly_goal(&db, 51).await.is_err());
}

#[tokio::test]
async fn counts_people_in_touch_this_week_not_notes_or_old_contact() {
    let db = Db::open_in_memory().await.unwrap();
    let a = person(&db, "A").await;
    let b = person(&db, "B").await;
    let c = person(&db, "C").await;
    add_touch(&db, a, Kind::Call, "x", "2026-12-08T10:00:00Z")
        .await
        .unwrap();
    add_touch(&db, a, Kind::Email, "y", "2026-12-09T10:00:00Z")
        .await
        .unwrap();
    add_touch(&db, b, Kind::Note, "just a note", "2026-12-09T10:00:00Z")
        .await
        .unwrap();
    add_touch(&db, c, Kind::Call, "old", "2026-11-01T10:00:00Z")
        .await
        .unwrap();
    assert_eq!(summary(&db, NOW).await.unwrap().this_week, 1);
}

#[tokio::test]
async fn funnel_and_recent_moves() {
    let db = Db::open_in_memory().await.unwrap();
    let co = ceevee_search::companies::find_or_create(&db, "Acme")
        .await
        .unwrap()
        .id;
    let a = person(&db, "A").await;
    let b = person(&db, "B").await;
    let c = person(&db, "C").await;
    let ra = record_ask(&db, a, co, "hi", "2026-12-05T00:00:00Z")
        .await
        .unwrap();
    let rb = record_ask(&db, b, co, "hi", "2026-12-06T00:00:00Z")
        .await
        .unwrap();
    record_ask(&db, c, co, "hi", "2026-10-01T00:00:00Z")
        .await
        .unwrap(); // too old
    set_outcome(&db, ra, "referred", NOW).await.unwrap();
    set_outcome(&db, rb, "replied", NOW).await.unwrap();
    sqlx::query(
        "INSERT INTO contact_change(contact_id, from_company, to_company, seen_at) VALUES (?, 'X', 'Y', '2026-12-01T00:00:00Z')",
    )
    .bind(a)
    .execute(db.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO contact_change(contact_id, from_company, to_company, seen_at) VALUES (?, 'P', 'Q', '2026-09-01T00:00:00Z')",
    )
    .bind(b)
    .execute(db.pool())
    .await
    .unwrap();
    let p = summary(&db, NOW).await.unwrap();
    assert_eq!((p.asked, p.replied, p.referred), (2, 2, 1));
    assert_eq!(p.moves.len(), 1);
    assert_eq!(p.moves[0].to_company.as_deref(), Some("Y"));
}
