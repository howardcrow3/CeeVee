use super::*;
use crate::postings::{self, Ingest, NewPosting};

fn crit() -> Criteria {
    Criteria {
        titles: vec!["product manager".into()],
        keywords: vec!["saas".into(), "roadmap".into()],
        locations: vec!["columbus".into()],
        remote_ok: true,
        avoid: vec!["intern".into()],
    }
}

#[test]
fn a_full_match_scores_100() {
    let s = score(
        &crit(),
        "Senior Product Manager",
        Some("Columbus, OH"),
        Some("Own the SaaS roadmap."),
    );
    assert_eq!(s.value, 100.0);
    assert!(s.reasons.iter().any(|r| r.contains("title matches")));
}

#[test]
fn partial_matches_share_the_score() {
    // Title yes, one of two keywords, remote instead of Columbus: 1 + 0.5 + 1 of 3 parts.
    let s = score(
        &crit(),
        "Product Manager",
        Some("Remote"),
        Some("Work on the roadmap."),
    );
    assert_eq!(s.value, 83.0);
    assert!(s
        .reasons
        .iter()
        .any(|r| r.contains("does not mention saas")));
}

#[test]
fn wrong_title_and_place_score_low() {
    let s = score(&crit(), "Warehouse Associate", Some("Dayton"), None);
    assert_eq!(s.value, 0.0);
}

#[test]
fn avoid_words_cap_the_score_and_match_whole_words() {
    let s = score(
        &crit(),
        "Product Manager Intern",
        Some("Columbus"),
        Some("saas roadmap"),
    );
    assert_eq!(s.value, 10.0);
    // "internal" is not "intern".
    let s = score(
        &crit(),
        "Product Manager",
        Some("Columbus"),
        Some("saas roadmap, internal tools"),
    );
    assert_eq!(s.value, 100.0);
}

#[test]
fn empty_criteria_score_zero() {
    assert_eq!(
        score(&Criteria::default(), "Anything", None, None).value,
        0.0
    );
}

#[tokio::test]
async fn categories_round_trip_and_names_are_unique() {
    let db = Db::open_in_memory().await.unwrap();
    let c = create(&db, "PM roles", &crit()).await.unwrap();
    assert_eq!(get(&db, c.id).await.unwrap().criteria, crit());
    assert!(create(&db, "PM roles", &crit()).await.is_err());
    assert!(create(&db, " ", &crit()).await.is_err());
}

#[tokio::test]
async fn scoring_a_posting_fills_the_category_rows_and_best_score() {
    let db = Db::open_in_memory().await.unwrap();
    let pm = create(&db, "PM", &crit()).await.unwrap();
    let other = create(
        &db,
        "Design",
        &Criteria {
            titles: vec!["designer".into()],
            ..Criteria::default()
        },
    )
    .await
    .unwrap();
    let off = create(&db, "Off", &crit()).await.unwrap();
    set_active(&db, off.id, false).await.unwrap();
    let Ingest::New(id) = postings::ingest(
        &db,
        &NewPosting {
            source: "x".into(),
            url: "https://a/1".into(),
            company: "Acme".into(),
            title: "Product Manager".into(),
            location: Some("Columbus".into()),
            body: Some("saas roadmap".into()),
            ..NewPosting::default()
        },
        "2026-11-01T10:00:00Z",
    )
    .await
    .unwrap() else {
        panic!()
    };
    let scores = score_posting(&db, id).await.unwrap();
    assert_eq!(scores.len(), 2);
    assert_eq!(postings::get(&db, id).await.unwrap().score, Some(100.0));
    // Re-scoring updates rather than duplicating.
    score_posting(&db, id).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posting_category")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(n, 2);
    let listed = postings::for_category(&db, pm.id, 50.0).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert!(postings::for_category(&db, other.id, 50.0)
        .await
        .unwrap()
        .is_empty());
    postings::set_state(&db, id, postings::State::Dismissed)
        .await
        .unwrap();
    assert!(postings::for_category(&db, pm.id, 0.0)
        .await
        .unwrap()
        .is_empty());
}
