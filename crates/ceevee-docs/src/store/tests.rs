use super::*;

async fn letter(db: &Db) -> i64 {
    create(
        db,
        DocKind::CoverLetter,
        "Letter for Acme",
        Some(7),
        None,
        "Dear team,",
        "created",
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn creating_makes_the_first_version_current() {
    let db = Db::open_in_memory().await.unwrap();
    let id = letter(&db).await;
    let d = get(&db, id).await.unwrap();
    assert_eq!((d.kind, d.application_id), (DocKind::CoverLetter, Some(7)));
    let v = current(&db, id).await.unwrap();
    assert_eq!(
        (v.content.as_str(), v.cause.as_str()),
        ("Dear team,", "created")
    );
    assert_eq!(d.current_version_id, Some(v.id));
}

#[tokio::test]
async fn every_change_is_a_version_and_unchanged_saves_are_ignored() {
    let db = Db::open_in_memory().await.unwrap();
    let id = letter(&db).await;
    let v2 = add_version(&db, id, "Dear Acme team,", "edited by you", None)
        .await
        .unwrap();
    let same = add_version(&db, id, "Dear Acme team,", "edited by you", None)
        .await
        .unwrap();
    assert_eq!(v2, same);
    add_version(
        &db,
        id,
        "Dear Acme team, shorter.",
        "chat: make it shorter",
        Some("write_letter@1"),
    )
    .await
    .unwrap();
    let all = versions(&db, id).await.unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].created_with.as_deref(), Some("write_letter@1"));
    assert_eq!(all[2].content, "Dear team,");
}

#[tokio::test]
async fn undo_and_redo_walk_the_history_without_losing_it() {
    let db = Db::open_in_memory().await.unwrap();
    let id = letter(&db).await;
    add_version(&db, id, "B", "edit", None).await.unwrap();
    add_version(&db, id, "C", "edit", None).await.unwrap();
    assert_eq!(undo(&db, id).await.unwrap().content, "B");
    assert_eq!(undo(&db, id).await.unwrap().content, "Dear team,");
    assert!(undo(&db, id).await.is_err());
    assert_eq!(redo(&db, id).await.unwrap().content, "B");
    assert_eq!(current(&db, id).await.unwrap().content, "B");
    assert_eq!(versions(&db, id).await.unwrap().len(), 3);
    redo(&db, id).await.unwrap();
    assert!(redo(&db, id).await.is_err());
}

#[tokio::test]
async fn restore_rejects_a_version_of_another_document() {
    let db = Db::open_in_memory().await.unwrap();
    let a = letter(&db).await;
    let b = create(
        &db,
        DocKind::BaseResume,
        "Resume",
        None,
        None,
        "{}",
        "created",
    )
    .await
    .unwrap();
    let vb = current(&db, b).await.unwrap().id;
    assert!(matches!(restore(&db, a, vb).await, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn list_filters_rename_and_delete() {
    let db = Db::open_in_memory().await.unwrap();
    let a = letter(&db).await;
    let b = create(
        &db,
        DocKind::BaseResume,
        "Resume",
        None,
        None,
        "{}",
        "created",
    )
    .await
    .unwrap();
    assert_eq!(list(&db, None, None).await.unwrap().len(), 2);
    assert_eq!(
        list(&db, Some(DocKind::BaseResume), None).await.unwrap()[0].id,
        b
    );
    assert_eq!(list(&db, None, Some(7)).await.unwrap()[0].id, a);
    rename(&db, a, "Acme letter").await.unwrap();
    assert_eq!(get(&db, a).await.unwrap().title, "Acme letter");
    delete(&db, a).await.unwrap();
    assert!(matches!(get(&db, a).await, Err(Error::NotFound(_))));
    // Versions go with the document.
    let left: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM document_version WHERE document_id = ?")
            .bind(a)
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(left, 0);
    assert!(matches!(delete(&db, a).await, Err(Error::NotFound(_))));
}
