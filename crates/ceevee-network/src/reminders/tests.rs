use super::*;
use crate::people::{add, add_touch, set_tier, Kind, NewPerson};

async fn person(db: &Db, name: &str, tier: Tier) -> i64 {
    let id = add(
        db,
        &NewPerson {
            full_name: name.into(),
            ..NewPerson::default()
        },
        "manual",
    )
    .await
    .unwrap();
    set_tier(db, id, tier).await.unwrap();
    // Pretend they were added a year ago.
    sqlx::query("UPDATE contact SET created_at = '2025-01-01T00:00:00Z' WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .unwrap();
    id
}

const NOW: &str = "2026-01-01T00:00:00Z";

#[tokio::test]
async fn due_depends_on_closeness_and_last_contact() {
    let db = Db::open_in_memory().await.unwrap();
    let close = person(&db, "Close One", Tier::Close).await;
    let weak = person(&db, "Weak One", Tier::Weak).await;
    // Close person was spoken to 10 days ago: not due. Weak person never contacted: due.
    add_touch(&db, close, Kind::Call, "caught up", "2025-12-22T00:00:00Z")
        .await
        .unwrap();
    let d = due(&db, NOW, 10).await.unwrap();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].id, weak);
    assert!(d[0].never_contacted);
    assert!(d[0].days_silent > 300);
    // Close person is due 31 days later.
    let d = due(&db, "2026-02-01T00:00:00Z", 10).await.unwrap();
    assert!(d.iter().any(|x| x.id == close && !x.never_contacted));
}

#[tokio::test]
async fn a_note_does_not_reset_the_clock() {
    let db = Db::open_in_memory().await.unwrap();
    let id = person(&db, "Ada", Tier::Close).await;
    add_touch(
        &db,
        id,
        Kind::Note,
        "thought about her",
        "2025-12-30T00:00:00Z",
    )
    .await
    .unwrap();
    assert_eq!(due(&db, NOW, 10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn snooze_hides_until_the_date_and_most_overdue_comes_first() {
    let db = Db::open_in_memory().await.unwrap();
    let a = person(&db, "Aaa", Tier::Close).await;
    let b = person(&db, "Bbb", Tier::Weak).await;
    let d = due(&db, NOW, 10).await.unwrap();
    // Close is 335 days silent against a 30 day rule: more overdue than weak at 180.
    assert_eq!(d[0].id, a);
    snooze(&db, a, Some("2026-03-01T00:00:00Z")).await.unwrap();
    let d = due(&db, NOW, 10).await.unwrap();
    assert_eq!(d.iter().map(|x| x.id).collect::<Vec<_>>(), vec![b]);
    assert_eq!(due(&db, "2026-03-02T00:00:00Z", 10).await.unwrap().len(), 2);
    snooze(&db, a, None).await.unwrap();
    assert_eq!(due(&db, NOW, 10).await.unwrap().len(), 2);
    assert!(snooze(&db, 9999, None).await.is_err());
    assert_eq!(due(&db, NOW, 1).await.unwrap().len(), 1);
}
