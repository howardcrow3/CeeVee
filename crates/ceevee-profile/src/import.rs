//! Turn the output of the `import_resume` task into reviewable changes.

use ceevee_core::Result;
use ceevee_db::Db;
use serde::Deserialize;

use crate::repo::{self, Change, Entry, EntryKind};

/// The `import_resume` task's answer (see `crates/ceevee-ai/prompts/import_resume.schema.json`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResumeDraft {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub work: Vec<Work>,
    #[serde(default)]
    pub education: Vec<School>,
    #[serde(default)]
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Work {
    pub company: String,
    pub title: String,
    pub start: Option<String>,
    pub end: Option<String>,
    #[serde(default)]
    pub current: bool,
    #[serde(default)]
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct School {
    pub school: String,
    pub degree: Option<String>,
    pub field: Option<String>,
    pub end: Option<String>,
}

fn clean(s: &Option<String>) -> Option<String> {
    s.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// Compare a draft with the current profile and list what would change. Nothing is written.
///
/// - A contact field the draft has and the profile lacks, or has differently, becomes `SetBasic`
///   (with the old value, so the review screen can show both).
/// - Jobs and schools the profile does not already have become `AddEntry`.
/// - Skills not already present (ignoring case) become `AddSkill`.
pub async fn plan_import(db: &Db, draft: &ResumeDraft) -> Result<Vec<Change>> {
    let mut out = Vec::new();

    let current = repo::get_basics(db).await?;
    let basics = [
        (
            "full_name",
            clean(&draft.name),
            Some(current.full_name.clone()).filter(|s| !s.is_empty()),
        ),
        ("email", clean(&draft.email), current.email.clone()),
        ("phone", clean(&draft.phone), current.phone.clone()),
        ("location", clean(&draft.location), current.location.clone()),
        ("summary", clean(&draft.summary), current.summary.clone()),
    ];
    for (field, new, old) in basics {
        if let Some(new) = new {
            if old.as_deref() != Some(new.as_str()) {
                out.push(Change::SetBasic {
                    field: field.into(),
                    old,
                    new,
                });
            }
        }
    }

    let jobs = repo::list_entries(db, EntryKind::Experience).await?;
    for w in &draft.work {
        let entry = Entry {
            id: None,
            kind: EntryKind::Experience,
            title: w.title.trim().to_owned(),
            organization: Some(w.company.trim().to_owned()),
            location: None,
            start_date: clean(&w.start),
            // A current job has no end date, whatever the model put there.
            end_date: if w.current { None } else { clean(&w.end) },
            bullets: w
                .highlights
                .iter()
                .map(|h| h.trim().to_owned())
                .filter(|h| !h.is_empty())
                .collect(),
        };
        if !entry.title.is_empty() && !jobs.iter().any(|j| j.same_as(&entry)) {
            out.push(Change::AddEntry { entry });
        }
    }

    let schools = repo::list_entries(db, EntryKind::Education).await?;
    for s in &draft.education {
        let title = [clean(&s.degree), clean(&s.field)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
        let entry = Entry {
            id: None,
            kind: EntryKind::Education,
            title: if title.is_empty() {
                s.school.trim().to_owned()
            } else {
                title
            },
            organization: Some(s.school.trim().to_owned()),
            location: None,
            start_date: None,
            end_date: clean(&s.end),
            bullets: vec![],
        };
        if !entry.title.is_empty() && !schools.iter().any(|j| j.same_as(&entry)) {
            out.push(Change::AddEntry { entry });
        }
    }

    let have: Vec<String> = repo::list_skills(db)
        .await?
        .into_iter()
        .map(|s| s.to_lowercase())
        .collect();
    let mut seen = have;
    for s in &draft.skills {
        let name = s.trim();
        if !name.is_empty() && !seen.contains(&name.to_lowercase()) {
            seen.push(name.to_lowercase());
            out.push(Change::AddSkill {
                name: name.to_owned(),
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
