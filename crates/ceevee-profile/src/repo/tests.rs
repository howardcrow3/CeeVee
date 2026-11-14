use super::*;

fn job(title: &str, org: &str, start: &str) -> Entry {
    Entry {
        id: None,
        kind: EntryKind::Experience,
        title: title.into(),
        organization: Some(org.into()),
        location: None,
        start_date: Some(start.into()),
        end_date: None,
        bullets: vec!["Did a thing".into()],
    }
}

#[tokio::test]
async fn basics_start_empty_and_round_trip() {
    let db = Db::open_in_memory().await.unwrap();
    assert_eq!(get_basics(&db).await.unwrap(), Basics::default());
    let b = Basics {
        full_name: "Maria Lopez".into(),
        email: Some("m@example.com".into()),
        ..Basics::default()
    };
    set_basics(&db, &b).await.unwrap();
    assert_eq!(get_basics(&db).await.unwrap(), b);
    let b2 = Basics {
        phone: Some("614".into()),
        ..b.clone()
    };
    set_basics(&db, &b2).await.unwrap();
    assert_eq!(get_basics(&db).await.unwrap(), b2);
}

#[tokio::test]
async fn entries_sort_newest_first_and_delete() {
    let db = Db::open_in_memory().await.unwrap();
    add_entry(&db, &job("Analyst", "Harbor", "2018-06"))
        .await
        .unwrap();
    let newer = add_entry(&db, &job("Senior Analyst", "Northfield", "2021-03"))
        .await
        .unwrap();
    let list = list_entries(&db, EntryKind::Experience).await.unwrap();
    assert_eq!(list[0].title, "Senior Analyst");
    assert_eq!(list[0].bullets, vec!["Did a thing"]);
    assert!(list_entries(&db, EntryKind::Education)
        .await
        .unwrap()
        .is_empty());
    delete_entry(&db, newer).await.unwrap();
    assert_eq!(
        list_entries(&db, EntryKind::Experience)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(matches!(
        delete_entry(&db, newer).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn skills_ignore_case_duplicates() {
    let db = Db::open_in_memory().await.unwrap();
    assert!(add_skill(&db, "SQL").await.unwrap());
    assert!(!add_skill(&db, " sql ").await.unwrap());
    assert!(add_skill(&db, "Tableau").await.unwrap());
    assert!(add_skill(&db, "  ").await.is_err());
    assert_eq!(list_skills(&db).await.unwrap(), vec!["SQL", "Tableau"]);
}

#[tokio::test]
async fn facts_upsert_and_keep_the_sensitive_flag() {
    let db = Db::open_in_memory().await.unwrap();
    assert!(get_fact(&db, "work_auth").await.unwrap().is_none());
    set_fact(
        &db,
        &Fact {
            question_key: "work_auth".into(),
            answer: "yes".into(),
            sensitive: false,
        },
    )
    .await
    .unwrap();
    set_fact(
        &db,
        &Fact {
            question_key: "salary".into(),
            answer: "90000".into(),
            sensitive: true,
        },
    )
    .await
    .unwrap();
    set_fact(
        &db,
        &Fact {
            question_key: "work_auth".into(),
            answer: "no".into(),
            sensitive: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        get_fact(&db, "work_auth").await.unwrap().unwrap().answer,
        "no"
    );
    assert!(get_fact(&db, "salary").await.unwrap().unwrap().sensitive);
}

#[tokio::test]
async fn applying_changes_is_idempotent() {
    let db = Db::open_in_memory().await.unwrap();
    let add = Change::AddEntry {
        entry: job("Analyst", "Harbor", "2018-06"),
    };
    apply(&db, &add).await.unwrap();
    apply(&db, &add).await.unwrap();
    assert_eq!(
        list_entries(&db, EntryKind::Experience)
            .await
            .unwrap()
            .len(),
        1
    );
    apply(&db, &Change::AddSkill { name: "SQL".into() })
        .await
        .unwrap();
    apply(&db, &Change::AddSkill { name: "sql".into() })
        .await
        .unwrap();
    assert_eq!(list_skills(&db).await.unwrap().len(), 1);
    apply(
        &db,
        &Change::SetBasic {
            field: "phone".into(),
            old: None,
            new: "614".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(get_basics(&db).await.unwrap().phone.as_deref(), Some("614"));
    let bad = apply(
        &db,
        &Change::SetBasic {
            field: "shoe_size".into(),
            old: None,
            new: "9".into(),
        },
    )
    .await;
    assert!(matches!(bad, Err(Error::Invalid(_))));
}

#[test]
fn changes_round_trip_through_json_for_proposals() {
    let c = Change::AddSkill { name: "SQL".into() };
    let j = serde_json::to_string(&c).unwrap();
    assert!(j.contains("\"change\":\"add_skill\""));
    assert_eq!(serde_json::from_str::<Change>(&j).unwrap(), c);
}
