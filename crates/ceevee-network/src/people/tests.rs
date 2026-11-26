use super::*;

fn ada() -> NewPerson {
    NewPerson {
        full_name: "Ada Lovelace".into(),
        email: Some("ada@example.com".into()),
        company: Some("Acme".into()),
        title: Some("Engineer".into()),
        tags: vec!["Mentor".into(), " mentor ".into(), "".into()],
        ..NewPerson::default()
    }
}

#[tokio::test]
async fn adds_and_reads_back_with_company_and_clean_tags() {
    let db = Db::open_in_memory().await.unwrap();
    let id = add(&db, &ada(), "manual").await.unwrap();
    let p = get(&db, id).await.unwrap();
    assert_eq!(p.full_name, "Ada Lovelace");
    assert_eq!(p.company.as_deref(), Some("Acme"));
    assert_eq!(p.title.as_deref(), Some("Engineer"));
    assert_eq!(p.tags, vec!["mentor"]);
    assert_eq!(p.tier, Tier::Warm);
    assert!(add(&db, &NewPerson::default(), "manual").await.is_err());
}

#[tokio::test]
async fn lists_closest_first_and_filters() {
    let db = Db::open_in_memory().await.unwrap();
    let a = add(&db, &ada(), "manual").await.unwrap();
    let b = add(
        &db,
        &NewPerson {
            full_name: "Grace Hopper".into(),
            company: Some("Navy".into()),
            ..NewPerson::default()
        },
        "manual",
    )
    .await
    .unwrap();
    set_tier(&db, b, Tier::Close).await.unwrap();
    let all = list(&db, None, None).await.unwrap();
    assert_eq!(all[0].id, b);
    assert_eq!(all[1].id, a);
    assert_eq!(list(&db, Some("acme"), None).await.unwrap().len(), 1);
    assert_eq!(list(&db, None, Some(Tier::Close)).await.unwrap().len(), 1);
    assert!(list(&db, Some("nobody"), None).await.unwrap().is_empty());
}

#[tokio::test]
async fn touches_build_a_timeline_and_set_last_contacted() {
    let db = Db::open_in_memory().await.unwrap();
    let id = add(&db, &ada(), "manual").await.unwrap();
    add_touch(&db, id, Kind::Note, "Likes Rust", "2026-11-20T10:00:00Z")
        .await
        .unwrap();
    // A note is not contact.
    assert_eq!(get(&db, id).await.unwrap().last_contacted, None);
    add_touch(&db, id, Kind::Meeting, "Coffee", "2026-11-22T10:00:00Z")
        .await
        .unwrap();
    add_touch(&db, id, Kind::Email, "Thanks", "2026-11-21T10:00:00Z")
        .await
        .unwrap();
    let p = get(&db, id).await.unwrap();
    assert_eq!(p.last_contacted.as_deref(), Some("2026-11-22T10:00:00Z"));
    let t = timeline(&db, id).await.unwrap();
    assert_eq!(t.len(), 3);
    assert_eq!(t[0].kind, Kind::Meeting);
    assert!(add_touch(&db, id, Kind::Note, "  ", "2026-11-22T10:00:00Z")
        .await
        .is_err());
    assert!(add_touch(&db, 999, Kind::Note, "x", "2026-11-22T10:00:00Z")
        .await
        .is_err());
}

#[tokio::test]
async fn finds_the_same_person_by_email_url_or_name_and_company() {
    let db = Db::open_in_memory().await.unwrap();
    let mut p = ada();
    p.profile_url = Some("https://linkedin.com/in/ada".into());
    let id = add(&db, &p, "manual").await.unwrap();
    let by_email = NewPerson {
        full_name: "A. Lovelace".into(),
        email: Some("ADA@example.com".into()),
        ..NewPerson::default()
    };
    assert_eq!(find_match(&db, &by_email).await.unwrap(), Some(id));
    let by_url = NewPerson {
        full_name: "Someone".into(),
        profile_url: Some("https://linkedin.com/in/ada".into()),
        ..NewPerson::default()
    };
    assert_eq!(find_match(&db, &by_url).await.unwrap(), Some(id));
    let by_name = NewPerson {
        full_name: "ada lovelace".into(),
        company: Some("Acme Inc".into()),
        ..NewPerson::default()
    };
    assert_eq!(find_match(&db, &by_name).await.unwrap(), Some(id));
    // Same name at a different company is a different person.
    let other = NewPerson {
        full_name: "Ada Lovelace".into(),
        company: Some("Globex".into()),
        ..NewPerson::default()
    };
    assert_eq!(find_match(&db, &other).await.unwrap(), None);
}

#[tokio::test]
async fn merging_keeps_everything_and_removes_the_duplicate() {
    let db = Db::open_in_memory().await.unwrap();
    let a = add(&db, &ada(), "manual").await.unwrap();
    let b = add(
        &db,
        &NewPerson {
            full_name: "Ada L".into(),
            phone: Some("555-0100".into()),
            notes: Some("Met at RustConf".into()),
            tags: vec!["speaker".into()],
            ..NewPerson::default()
        },
        "vcard",
    )
    .await
    .unwrap();
    add_touch(&db, b, Kind::Call, "Intro call", "2026-11-22T10:00:00Z")
        .await
        .unwrap();
    merge(&db, a, b).await.unwrap();
    let p = get(&db, a).await.unwrap();
    assert_eq!(p.phone.as_deref(), Some("555-0100"));
    assert_eq!(p.notes.as_deref(), Some("Met at RustConf"));
    assert_eq!(p.tags, vec!["mentor", "speaker"]);
    assert_eq!(p.last_contacted.as_deref(), Some("2026-11-22T10:00:00Z"));
    assert_eq!(timeline(&db, a).await.unwrap().len(), 1);
    assert!(get(&db, b).await.is_err());
    assert!(merge(&db, a, a).await.is_err());
}

#[tokio::test]
async fn update_and_delete() {
    let db = Db::open_in_memory().await.unwrap();
    let id = add(&db, &ada(), "manual").await.unwrap();
    let mut p = ada();
    p.full_name = "Ada King".into();
    p.company = Some("Globex".into());
    update(&db, id, &p).await.unwrap();
    let got = get(&db, id).await.unwrap();
    assert_eq!(got.full_name, "Ada King");
    assert_eq!(got.company.as_deref(), Some("Globex"));
    delete(&db, id).await.unwrap();
    assert!(get(&db, id).await.is_err());
    assert!(update(&db, id, &p).await.is_err());
}
