use super::*;
use crate::companies::{self, Status};

fn p(source: &str, url: &str, company: &str, title: &str, loc: &str) -> NewPosting {
    NewPosting {
        source: source.into(),
        url: url.into(),
        company: company.into(),
        title: title.into(),
        location: Some(loc.into()),
        ..NewPosting::default()
    }
}

const T0: &str = "2026-11-01T10:00:00Z";
const T1: &str = "2026-11-02T10:00:00Z";

#[tokio::test]
async fn same_url_is_a_duplicate_and_moves_last_seen() {
    let db = Db::open_in_memory().await.unwrap();
    let a = ingest(&db, &p("x", "https://a/1", "Acme", "PM", "Columbus"), T0)
        .await
        .unwrap();
    let Ingest::New(id) = a else { panic!("{a:?}") };
    let b = ingest(&db, &p("x", "https://a/1", "Acme", "PM", "Columbus"), T1)
        .await
        .unwrap();
    assert_eq!(b, Ingest::Duplicate(id));
    let last: String = sqlx::query_scalar("SELECT last_seen FROM posting WHERE id = ?")
        .bind(id)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(last, T1);
}

#[tokio::test]
async fn same_job_from_another_board_is_a_duplicate() {
    let db = Db::open_in_memory().await.unwrap();
    let Ingest::New(id) = ingest(
        &db,
        &p(
            "boardA",
            "https://a/1",
            "Acme, Inc.",
            "Sr. Product Manager",
            "Columbus, OH",
        ),
        T0,
    )
    .await
    .unwrap() else {
        panic!()
    };
    let again = ingest(
        &db,
        &p(
            "boardB",
            "https://b/9",
            "ACME",
            "sr product manager",
            "columbus oh",
        ),
        T1,
    )
    .await
    .unwrap();
    assert_eq!(again, Ingest::Duplicate(id));
    // A different title at the same company is a different job.
    let other = ingest(
        &db,
        &p("boardB", "https://b/10", "Acme", "Designer", "Columbus, OH"),
        T1,
    )
    .await
    .unwrap();
    assert!(matches!(other, Ingest::New(_)));
}

#[tokio::test]
async fn blocked_companies_are_skipped() {
    let db = Db::open_in_memory().await.unwrap();
    let c = companies::find_or_create(&db, "Evil Corp").await.unwrap();
    companies::set_status(&db, c.id, Status::Blocked)
        .await
        .unwrap();
    let r = ingest(
        &db,
        &p("x", "https://e/1", "Evil Corp.", "PM", "Remote"),
        T0,
    )
    .await
    .unwrap();
    assert_eq!(r, Ingest::Blocked);
}

#[tokio::test]
async fn aliases_match_and_bad_input_is_refused() {
    let db = Db::open_in_memory().await.unwrap();
    let c = companies::find_or_create(&db, "Alphabet").await.unwrap();
    companies::add_alias(&db, c.id, "Google LLC").await.unwrap();
    assert_eq!(
        companies::find_or_create(&db, "google").await.unwrap().id,
        c.id
    );
    assert!(ingest(&db, &p("x", "", "A", "B", "C"), T0).await.is_err());
    assert!(companies::find_or_create(&db, "  ").await.is_err());
}

#[tokio::test]
async fn state_changes() {
    let db = Db::open_in_memory().await.unwrap();
    let Ingest::New(id) = ingest(&db, &p("x", "https://a/1", "Acme", "PM", "Columbus"), T0)
        .await
        .unwrap()
    else {
        panic!()
    };
    set_state(&db, id, State::Dismissed).await.unwrap();
    assert!(set_state(&db, 999, State::Saved).await.is_err());
}
