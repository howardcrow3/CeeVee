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
    let all = list_facts(&db).await.unwrap();
    assert_eq!(
        all.iter()
            .map(|f| f.question_key.as_str())
            .collect::<Vec<_>>(),
        vec!["salary", "work_auth"]
    );
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

#[test]
fn changes_read_as_plain_sentences() {
    let c = Change::SetBasic {
        field: "location".into(),
        old: Some("Dayton, OH".into()),
        new: "Columbus, OH".into(),
    };
    assert_eq!(
        describe(&c),
        "Location: change \"Dayton, OH\" to \"Columbus, OH\""
    );
    let c = Change::SetBasic {
        field: "email".into(),
        old: None,
        new: "a@b.co".into(),
    };
    assert_eq!(describe(&c), "Email: set to \"a@b.co\"");
    assert_eq!(
        describe(&Change::AddSkill { name: "SQL".into() }),
        "Add skill: SQL"
    );
}

#[tokio::test]
async fn deleting_a_missing_entry_is_an_error_not_a_silent_success() {
    let db = Db::open_in_memory().await.unwrap();
    let err = delete_entry(&db, 999).await.unwrap_err();
    assert!(matches!(err, Error::NotFound(_)), "{err}");
}

#[tokio::test]
async fn skills_are_trimmed_and_blank_ones_refused() {
    let db = Db::open_in_memory().await.unwrap();
    assert!(add_skill(&db, "  Rust  ").await.unwrap());
    assert!(!add_skill(&db, "rust").await.unwrap());
    assert!(matches!(
        add_skill(&db, "   ").await.unwrap_err(),
        Error::Invalid(_)
    ));
    assert_eq!(list_skills(&db).await.unwrap(), vec!["Rust".to_string()]);
}

#[tokio::test]
async fn entries_of_different_kinds_are_listed_apart_and_keep_their_bullets() {
    let db = Db::open_in_memory().await.unwrap();
    let mut j = job("Engineer", "Acme", "2020-01");
    j.bullets = vec![
        "Built café menus – 40% faster".into(),
        "Led \"team\"".into(),
    ];
    add_entry(&db, &j).await.unwrap();
    let mut school = job("BSc", "State U", "2014-09");
    school.kind = EntryKind::Education;
    add_entry(&db, &school).await.unwrap();

    let jobs = list_entries(&db, EntryKind::Experience).await.unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].bullets, j.bullets);
    let edu = list_entries(&db, EntryKind::Education).await.unwrap();
    assert_eq!(edu.len(), 1);
    assert_eq!(edu[0].title, "BSc");
    assert!(list_entries(&db, EntryKind::Project)
        .await
        .unwrap()
        .is_empty());
}
