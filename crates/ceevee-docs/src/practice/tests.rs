use super::*;

#[test]
fn questions_lose_numbering_blanks_and_repeats() {
    let got = clean(vec![
        "1. Why this role?".into(),
        "  ".into(),
        "why this role?".into(),
        "- Tell me about SQL.".into(),
    ]);
    assert_eq!(got, vec!["Why this role?", "Tell me about SQL."]);
}

async fn an_application(db: &Db) -> i64 {
    ceevee_db_application(db).await
}

async fn ceevee_db_application(db: &Db) -> i64 {
    sqlx::query_scalar("INSERT INTO application (status) VALUES ('interview') RETURNING id")
        .fetch_one(db.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn notes_stay_with_the_application_and_go_with_it() {
    let db = Db::open_in_memory().await.unwrap();
    let a = an_application(&db).await;
    let b = an_application(&db).await;
    save_note(&db, a, " Why us? ", "Because of SQL work.", "Add a result.")
        .await
        .unwrap();
    let second = save_note(&db, a, "Biggest win?", "Cut report prep.", "")
        .await
        .unwrap();
    save_note(&db, b, "Other?", "Other.", "").await.unwrap();
    let list = notes(&db, a).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].question, "Why us?");
    assert_eq!(list[1].id, second);
    delete_note(&db, second).await.unwrap();
    assert_eq!(notes(&db, a).await.unwrap().len(), 1);
    sqlx::query("DELETE FROM application WHERE id = ?")
        .bind(a)
        .execute(db.pool())
        .await
        .unwrap();
    assert!(notes(&db, a).await.unwrap().is_empty());
    assert_eq!(notes(&db, b).await.unwrap().len(), 1);
}

#[tokio::test]
async fn an_empty_note_is_refused() {
    let db = Db::open_in_memory().await.unwrap();
    let a = an_application(&db).await;
    assert!(save_note(&db, a, "", "x", "").await.is_err());
    assert!(save_note(&db, a, "q", "  ", "").await.is_err());
}
