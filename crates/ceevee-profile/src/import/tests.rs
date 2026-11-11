use serde_json::json;

use super::*;
use crate::repo::{apply, get_basics, list_entries, list_skills, Basics};

fn draft() -> ResumeDraft {
    serde_json::from_value(json!({
        "name": "Maria Lopez", "email": "maria@example.com", "phone": null,
        "location": "Columbus, OH", "summary": "Operations analyst.",
        "work": [
            {"company": "Northfield", "title": "Senior Analyst", "start": "2021-03", "end": "2024-01", "current": true,
             "highlights": ["Built dashboards", "  "]},
            {"company": "Harbor", "title": "Analyst", "start": "2018-06", "end": "2021-02", "current": false, "highlights": []}
        ],
        "education": [{"school": "Ohio State", "degree": "B.S.", "field": "Industrial Engineering", "end": "2018"}],
        "skills": ["SQL", "sql", "Tableau"]
    }))
    .unwrap()
}

#[tokio::test]
async fn an_empty_profile_gets_everything_once() {
    let db = Db::open_in_memory().await.unwrap();
    let plan = plan_import(&db, &draft()).await.unwrap();
    let basics = plan
        .iter()
        .filter(|c| matches!(c, Change::SetBasic { .. }))
        .count();
    let entries = plan
        .iter()
        .filter(|c| matches!(c, Change::AddEntry { .. }))
        .count();
    let skills: Vec<_> = plan
        .iter()
        .filter_map(|c| {
            if let Change::AddSkill { name } = c {
                Some(name.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!((basics, entries), (4, 3)); // phone is null, so no change for it
    assert_eq!(skills, vec!["SQL", "Tableau"]); // the repeated "sql" is dropped
                                                // Nothing was written by planning.
    assert!(list_entries(&db, EntryKind::Experience)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn a_current_job_has_no_end_date_and_blank_bullets_are_dropped() {
    let db = Db::open_in_memory().await.unwrap();
    let plan = plan_import(&db, &draft()).await.unwrap();
    let senior = plan
        .iter()
        .find_map(|c| match c {
            Change::AddEntry { entry } if entry.title == "Senior Analyst" => Some(entry),
            _ => None,
        })
        .unwrap();
    assert_eq!(senior.end_date, None);
    assert_eq!(senior.bullets, vec!["Built dashboards"]);
}

#[tokio::test]
async fn re_importing_the_same_resume_proposes_nothing() {
    let db = Db::open_in_memory().await.unwrap();
    for c in plan_import(&db, &draft()).await.unwrap() {
        apply(&db, &c).await.unwrap();
    }
    assert_eq!(get_basics(&db).await.unwrap().full_name, "Maria Lopez");
    assert_eq!(
        list_entries(&db, EntryKind::Experience)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(list_skills(&db).await.unwrap(), vec!["SQL", "Tableau"]);
    assert!(plan_import(&db, &draft()).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_different_value_is_a_change_that_shows_the_old_one() {
    let db = Db::open_in_memory().await.unwrap();
    crate::repo::set_basics(
        &db,
        &Basics {
            full_name: "Maria Lopez".into(),
            location: Some("Dayton, OH".into()),
            ..Basics::default()
        },
    )
    .await
    .unwrap();
    let plan = plan_import(&db, &draft()).await.unwrap();
    let loc = plan
        .iter()
        .find(|c| matches!(c, Change::SetBasic { field, .. } if field == "location"))
        .unwrap();
    assert_eq!(
        loc,
        &Change::SetBasic {
            field: "location".into(),
            old: Some("Dayton, OH".into()),
            new: "Columbus, OH".into()
        }
    );
    // The name already matches, so it is not proposed.
    assert!(!plan
        .iter()
        .any(|c| matches!(c, Change::SetBasic { field, .. } if field == "full_name")));
}
