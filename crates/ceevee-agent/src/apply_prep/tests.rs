use super::*;
use ceevee_profile::repo::{set_basics, set_fact, Basics, Fact};
use ceevee_search::postings::{ingest, NewPosting};

async fn posting(db: &Db) -> i64 {
    let p = NewPosting {
        source: "greenhouse".into(),
        url: "https://boards.greenhouse.io/acme/jobs/7".into(),
        company: "Acme".into(),
        title: "Analyst".into(),
        location: None,
        level: None,
        pay: None,
        body: None,
    };
    match ingest(db, &p, "2026-11-15T00:00:00Z").await.unwrap() {
        postings::Ingest::New(id) => id,
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn needs_a_name_first() {
    let db = Db::open_in_memory().await.unwrap();
    let id = posting(&db).await;
    let dir = tempfile::tempdir().unwrap();
    let e = gather(&db, id, dir.path(), Paper::UsLetter)
        .await
        .err()
        .unwrap();
    assert!(e.to_string().contains("name"));
}

#[tokio::test]
async fn gathers_answers_and_renders_the_resume() {
    let db = Db::open_in_memory().await.unwrap();
    let id = posting(&db).await;
    set_basics(
        &db,
        &Basics {
            full_name: "Ada Lovelace".into(),
            email: Some("ada@example.com".into()),
            phone: None,
            location: Some("London, UK".into()),
            summary: None,
        },
    )
    .await
    .unwrap();
    set_fact(
        &db,
        &Fact {
            question_key: "authorized to work".into(),
            answer: "Yes".into(),
            sensitive: true,
        },
    )
    .await
    .unwrap();
    store::create(
        &db,
        DocKind::BaseResume,
        "Base: Ada / Résumé",
        None,
        None,
        r#"{"basics":{"name":"Ada Lovelace"}}"#,
        "test",
    )
    .await
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let i = gather(&db, id, dir.path(), Paper::UsLetter).await.unwrap();
    assert_eq!(i.url, "https://boards.greenhouse.io/acme/jobs/7");
    assert_eq!(i.answers.first_name, "Ada");
    assert_eq!(i.answers.city, "London");
    assert_eq!(
        i.answers.saved_for("Authorized to work in the US?"),
        Some("Yes")
    );
    let pdf = std::fs::read(i.answers.resume_pdf.unwrap()).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert_eq!(i.resume_title.as_deref(), Some("Base: Ada / Résumé"));
}

#[tokio::test]
async fn works_without_any_resume_yet() {
    let db = Db::open_in_memory().await.unwrap();
    let id = posting(&db).await;
    set_basics(
        &db,
        &Basics {
            full_name: "Ada".into(),
            ..Basics::default()
        },
    )
    .await
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let i = gather(&db, id, dir.path(), Paper::A4).await.unwrap();
    assert!(i.answers.resume_pdf.is_none() && i.resume_title.is_none());
}

#[tokio::test]
async fn attaches_the_cover_letter_and_resume_saved_for_this_job() {
    let db = Db::open_in_memory().await.unwrap();
    let id = posting(&db).await;
    set_basics(
        &db,
        &Basics {
            full_name: "Ada Lovelace".into(),
            ..Basics::default()
        },
    )
    .await
    .unwrap();
    let app = ceevee_search::tracking::create(
        &db,
        &ceevee_search::tracking::NewApplication {
            posting_id: Some(id),
            ..Default::default()
        },
        "2026-11-15T00:00:00Z",
    )
    .await
    .unwrap();
    // Newer documents for some other job must not be picked.
    store::create(
        &db,
        DocKind::CoverLetter,
        "Letter: this job",
        Some(app.id),
        None,
        "Dear team,\n\nHello.",
        "test",
    )
    .await
    .unwrap();
    store::create(
        &db,
        DocKind::TailoredResume,
        "Resume: this job",
        Some(app.id),
        None,
        r#"{"basics":{"name":"Ada Lovelace"}}"#,
        "test",
    )
    .await
    .unwrap();
    store::create(
        &db,
        DocKind::TailoredResume,
        "Resume: other job",
        None,
        None,
        r#"{"basics":{"name":"Ada Lovelace"}}"#,
        "test",
    )
    .await
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let i = gather(&db, id, dir.path(), Paper::UsLetter).await.unwrap();
    assert_eq!(i.resume_title.as_deref(), Some("Resume: this job"));
    assert_eq!(i.cover_letter_title.as_deref(), Some("Letter: this job"));
    let letter = std::fs::read(i.answers.cover_letter_pdf.unwrap()).unwrap();
    assert!(letter.starts_with(b"%PDF"));
}
