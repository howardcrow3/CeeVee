//! The profile written out as a JSON-Resume-style document, the form CeeVee stores base resumes in.

use ceevee_core::Result;
use ceevee_db::Db;
use serde_json::{json, Value};

use crate::repo::{get_basics, get_links, list_entries, list_skills, Entry, EntryKind};

fn work(e: &Entry) -> Value {
    json!({
        "name": e.organization.clone().unwrap_or_default(),
        "position": e.title,
        "startDate": e.start_date.clone().unwrap_or_default(),
        "endDate": e.end_date.clone().unwrap_or_default(),
        "highlights": e.bullets,
    })
}

fn education(e: &Entry) -> Value {
    // An education entry keeps the degree in the title and the field after a comma, if any.
    let (study, area) = e
        .title
        .split_once(',')
        .map_or((e.title.as_str(), ""), |(a, b)| (a, b.trim()));
    json!({
        "institution": e.organization.clone().unwrap_or_default(),
        "studyType": study.trim(),
        "area": area,
        "endDate": e.end_date.clone().unwrap_or_default(),
    })
}

/// The whole profile as a resume document.
pub async fn profile_resume(db: &Db) -> Result<Value> {
    let b = get_basics(db).await?;
    let links = get_links(db).await?;
    let jobs = list_entries(db, EntryKind::Experience).await?;
    let schools = list_entries(db, EntryKind::Education).await?;
    let skills = list_skills(db).await?;
    Ok(json!({
        "basics": {
            "name": b.full_name,
            "email": b.email.unwrap_or_default(),
            "phone": b.phone.unwrap_or_default(),
            "location": b.location.unwrap_or_default(),
            "summary": b.summary.unwrap_or_default(),
            "profiles": links.iter().map(|u| json!({ "url": u })).collect::<Vec<_>>(),
        },
        "work": jobs.iter().map(work).collect::<Vec<_>>(),
        "education": schools.iter().map(education).collect::<Vec<_>>(),
        "skills": skills.iter().map(|s| json!({ "name": s })).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::{add_entry, add_skill, set_basics, Basics};

    #[tokio::test]
    async fn writes_the_profile_as_a_resume() {
        let db = Db::open_in_memory().await.unwrap();
        set_basics(
            &db,
            &Basics {
                full_name: "Maria Lopez".into(),
                email: Some("m@example.com".into()),
                ..Basics::default()
            },
        )
        .await
        .unwrap();
        add_entry(
            &db,
            &Entry {
                id: None,
                kind: EntryKind::Experience,
                title: "Senior Analyst".into(),
                organization: Some("Northfield".into()),
                location: None,
                start_date: Some("2021-03".into()),
                end_date: None,
                bullets: vec!["Built dashboards".into()],
            },
        )
        .await
        .unwrap();
        add_entry(
            &db,
            &Entry {
                id: None,
                kind: EntryKind::Education,
                title: "B.S., Industrial Engineering".into(),
                organization: Some("Ohio State".into()),
                location: None,
                start_date: None,
                end_date: Some("2018".into()),
                bullets: vec![],
            },
        )
        .await
        .unwrap();
        add_skill(&db, "SQL").await.unwrap();
        let r = profile_resume(&db).await.unwrap();
        assert_eq!(r["basics"]["name"], "Maria Lopez");
        assert_eq!(r["work"][0]["position"], "Senior Analyst");
        assert_eq!(r["work"][0]["highlights"][0], "Built dashboards");
        assert_eq!(r["education"][0]["studyType"], "B.S.");
        assert_eq!(r["education"][0]["area"], "Industrial Engineering");
        assert_eq!(r["skills"][0]["name"], "SQL");
    }
}
