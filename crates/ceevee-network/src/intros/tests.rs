use super::*;
use crate::people::{add, set_tier, NewPerson};

async fn person(db: &Db, name: &str, company: Option<&str>, tier: Tier) -> i64 {
    let id = add(
        db,
        &NewPerson {
            full_name: name.into(),
            company: company.map(str::to_string),
            title: Some("Engineer".into()),
            ..NewPerson::default()
        },
        "manual",
    )
    .await
    .unwrap();
    set_tier(db, id, tier).await.unwrap();
    id
}

async fn acme(db: &Db) -> i64 {
    ceevee_search::companies::find_or_create(db, "Acme")
        .await
        .unwrap()
        .id
}

const NOW: &str = "2026-12-01T00:00:00Z";

#[tokio::test]
async fn ranks_closer_people_first_and_finds_former_staff() {
    let db = Db::open_in_memory().await.unwrap();
    let co = acme(&db).await;
    let weak = person(&db, "Weak Wendy", Some("Acme"), Tier::Weak).await;
    let close = person(&db, "Close Carl", Some("Acme"), Tier::Close).await;
    let ex = person(&db, "Ex Eve", Some("Other Co"), Tier::Close).await;
    sqlx::query(
        "INSERT INTO contact_change(contact_id, from_company, from_title, to_company) VALUES (?, 'Acme', 'PM', 'Other Co')",
    )
    .bind(ex)
    .execute(db.pool())
    .await
    .unwrap();
    let _other = person(&db, "Unrelated Una", Some("Elsewhere"), Tier::Close).await;
    let p = paths(&db, co).await.unwrap();
    let names: Vec<&str> = p.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, vec!["Close Carl", "Ex Eve", "Weak Wendy"]);
    assert_eq!(p[0].contact_id, close);
    assert_eq!(p[1].link, Link::Worked);
    assert_eq!(p[2].contact_id, weak);
    assert!(paths(&db, 9999).await.is_err());
}

#[tokio::test]
async fn asking_is_recorded_and_moves_people_down() {
    let db = Db::open_in_memory().await.unwrap();
    let co = acme(&db).await;
    let a = person(&db, "Aaa", Some("Acme"), Tier::Close).await;
    let b = person(&db, "Bbb", Some("Acme"), Tier::Warm).await;
    let ask = record_ask(&db, a, co, "Hi Aaa", NOW).await.unwrap();
    let p = paths(&db, co).await.unwrap();
    assert_eq!(p[0].contact_id, b);
    assert_eq!(p[1].asked.as_deref(), Some("asked"));
    assert!(people::get(&db, a).await.unwrap().last_contacted.is_some());
    set_outcome(&db, ask, "referred", NOW).await.unwrap();
    assert_eq!(
        paths(&db, co).await.unwrap()[1].asked.as_deref(),
        Some("referred")
    );
    assert!(set_outcome(&db, ask, "maybe", NOW).await.is_err());
    assert!(set_outcome(&db, 999, "replied", NOW).await.is_err());
}

#[tokio::test]
async fn drafts_fit_the_situation() {
    let db = Db::open_in_memory().await.unwrap();
    let here = person(&db, "Ada Lovelace", Some("Acme"), Tier::Close).await;
    let gone = person(&db, "Grace Hopper", Some("Navy"), Tier::Warm).await;
    let d = draft(&db, here, "Acme", "Senior Engineer", Ask::Referral, NOW)
        .await
        .unwrap();
    assert!(d.starts_with("Hi Ada,"));
    assert!(d.contains("It has been a while"));
    assert!(d.contains("for the Senior Engineer role"));
    assert!(d.contains("referring me"));
    add_contacted(&db, here).await;
    let d = draft(&db, here, "Acme", "", Ask::Chat, NOW).await.unwrap();
    assert!(d.contains("Hope you are doing well"));
    assert!(!d.contains(" for the "));
    let d = draft(&db, gone, "Acme", "", Ask::Referral, NOW)
        .await
        .unwrap();
    assert!(d.contains("you used to work there"));
}

async fn add_contacted(db: &Db, id: i64) {
    people::add_touch(db, id, Kind::Call, "chat", "2026-11-20T00:00:00Z")
        .await
        .unwrap();
}
