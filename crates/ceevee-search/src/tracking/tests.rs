use super::*;

async fn app(db: &Db) -> Application {
    create(db, &NewApplication::default(), "2026-10-01T09:00:00Z")
        .await
        .unwrap()
}

#[tokio::test]
async fn new_applications_are_drafts_with_a_history() {
    let db = Db::open_in_memory().await.unwrap();
    let a = app(&db).await;
    assert_eq!(a.status, Status::Draft);
    let h = history(&db, a.id).await.unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].from, None);
    assert_eq!(h[0].to, Status::Draft);
}

#[tokio::test]
async fn status_changes_are_recorded_and_stamp_applied_at_once() {
    let db = Db::open_in_memory().await.unwrap();
    let a = app(&db).await;
    let a = set_status(
        &db,
        a.id,
        Status::Sent,
        Cause::Autofill,
        "2026-11-01T10:00:00Z",
    )
    .await
    .unwrap();
    assert_eq!(a.applied_at.as_deref(), Some("2026-11-01T10:00:00Z"));
    set_status(
        &db,
        a.id,
        Status::Screening,
        Cause::Email,
        "2026-11-03T10:00:00Z",
    )
    .await
    .unwrap();
    let h = history(&db, a.id).await.unwrap();
    assert_eq!(h.len(), 3);
    assert_eq!(h[2].from, Some(Status::Sent));
    assert_eq!(h[2].cause, "email");
}

#[tokio::test]
async fn invalid_moves_are_refused() {
    let db = Db::open_in_memory().await.unwrap();
    let a = app(&db).await;
    let now = "2026-11-01T10:00:00Z";
    // Cannot accept before an offer, or "move" to the same status.
    assert!(set_status(&db, a.id, Status::Accepted, Cause::Manual, now)
        .await
        .is_err());
    assert!(set_status(&db, a.id, Status::Draft, Cause::Manual, now)
        .await
        .is_err());
    set_status(&db, a.id, Status::Offer, Cause::Manual, now)
        .await
        .unwrap();
    set_status(&db, a.id, Status::Accepted, Cause::Manual, now)
        .await
        .unwrap();
    // A closed application can be reopened only as draft or sent.
    assert!(set_status(&db, a.id, Status::Interview, Cause::Manual, now)
        .await
        .is_err());
    set_status(&db, a.id, Status::Sent, Cause::Manual, now)
        .await
        .unwrap();
    assert!(set_status(&db, 9999, Status::Sent, Cause::Manual, now)
        .await
        .is_err());
}

#[tokio::test]
async fn list_filters_and_orders_by_board_position() {
    let db = Db::open_in_memory().await.unwrap();
    let a = app(&db).await;
    let b = app(&db).await;
    let c = app(&db).await;
    let now = "2026-11-01T10:00:00Z";
    set_status(&db, c.id, Status::Sent, Cause::Manual, now)
        .await
        .unwrap();
    let drafts = list(
        &db,
        &Filter {
            status: Some(Status::Draft),
            ..Filter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        drafts.iter().map(|x| x.id).collect::<Vec<_>>(),
        vec![a.id, b.id]
    );
    reorder(&db, Status::Draft, &[b.id, a.id]).await.unwrap();
    let drafts = list(
        &db,
        &Filter {
            status: Some(Status::Draft),
            ..Filter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        drafts.iter().map(|x| x.id).collect::<Vec<_>>(),
        vec![b.id, a.id]
    );
    // An id from another column is refused.
    assert!(reorder(&db, Status::Draft, &[c.id]).await.is_err());
}

#[tokio::test]
async fn reminders_find_quiet_open_applications_only() {
    let db = Db::open_in_memory().await.unwrap();
    let quiet = app(&db).await;
    let fresh = app(&db).await;
    let closed = app(&db).await;
    let draft = app(&db).await;
    set_status(
        &db,
        quiet.id,
        Status::Sent,
        Cause::Manual,
        "2026-10-20T10:00:00Z",
    )
    .await
    .unwrap();
    set_status(
        &db,
        fresh.id,
        Status::Sent,
        Cause::Manual,
        "2026-11-08T10:00:00Z",
    )
    .await
    .unwrap();
    set_status(
        &db,
        closed.id,
        Status::Sent,
        Cause::Manual,
        "2026-10-01T10:00:00Z",
    )
    .await
    .unwrap();
    set_status(
        &db,
        closed.id,
        Status::Rejected,
        Cause::Email,
        "2026-10-02T10:00:00Z",
    )
    .await
    .unwrap();
    let r = reminders(&db, "2026-11-11T10:00:00Z", 7).await.unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].application_id, quiet.id);
    assert_eq!(r[0].quiet_days, 22);
    let _ = draft;
}

#[tokio::test]
async fn next_interview_can_be_set_and_cleared() {
    let db = Db::open_in_memory().await.unwrap();
    let a = app(&db).await;
    set_next_interview(&db, a.id, Some("2026-11-20T15:00:00Z"))
        .await
        .unwrap();
    assert_eq!(
        get(&db, a.id).await.unwrap().next_interview_at.as_deref(),
        Some("2026-11-20T15:00:00Z")
    );
    set_next_interview(&db, a.id, None).await.unwrap();
    assert_eq!(get(&db, a.id).await.unwrap().next_interview_at, None);
}

#[tokio::test]
async fn adding_by_hand_creates_company_posting_and_status() {
    let db = Db::open_in_memory().await.unwrap();
    let a = add_manual(
        &db,
        " Acme Robotics ",
        "Product Manager",
        Status::Sent,
        "2026-11-23T10:00:00Z",
    )
    .await
    .unwrap();
    assert_eq!(a.status, Status::Sent);
    assert_eq!(a.applied_at.as_deref(), Some("2026-11-23T10:00:00Z"));
    let p = crate::postings::get(&db, a.posting_id.unwrap())
        .await
        .unwrap();
    assert_eq!(p.title, "Product Manager");
    let c = crate::companies::get(&db, p.company_id.unwrap())
        .await
        .unwrap();
    assert_eq!(c.name, "Acme Robotics");
}

#[tokio::test]
async fn adding_by_hand_needs_company_and_title() {
    let db = Db::open_in_memory().await.unwrap();
    assert!(
        add_manual(&db, "", "PM", Status::Draft, "2026-11-23T10:00:00Z")
            .await
            .is_err()
    );
    assert!(
        add_manual(&db, "Acme", "  ", Status::Draft, "2026-11-23T10:00:00Z")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn adding_by_hand_can_start_at_interview() {
    let db = Db::open_in_memory().await.unwrap();
    let a = add_manual(&db, "Acme", "PM", Status::Interview, "2026-11-23T10:00:00Z")
        .await
        .unwrap();
    assert_eq!(a.status, Status::Interview);
}
