//! Draft a reply to an employer email. Drafting always runs on its own; the draft is a saved
//! document the user reads and edits (by hand or in chat) before anything is sent.

use ceevee_ai::task::{run, DRAFT_REPLY};
use ceevee_ai::{ModelWorker, Priority};
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use ceevee_docs::fact_guard::{check, Flag};
use ceevee_docs::store::{self, DocKind};
use ceevee_profile::repo;
use serde::Deserialize;
use sqlx::Row;

use crate::label::clip;
use crate::store::{get_message, set_draft};

#[derive(Deserialize)]
struct Answer {
    reply: String,
}

/// A saved reply draft, with anything the fact guard did not find in the facts it was given.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplyDraft {
    pub document_id: i64,
    pub flags: Vec<Flag>,
}

/// The profile as plain text for a prompt: name, summary, jobs and skills.
pub async fn profile_text(db: &Db) -> Result<String> {
    let b = repo::get_basics(db).await?;
    let mut out = format!("{}\n", b.full_name);
    if let Some(s) = b.summary.as_deref().filter(|s| !s.trim().is_empty()) {
        out.push_str(s);
        out.push('\n');
    }
    for e in repo::list_entries(db, repo::EntryKind::Experience).await? {
        out.push_str(&format!(
            "{} at {} ({} to {}). {}\n",
            e.title,
            e.organization.unwrap_or_default(),
            e.start_date.unwrap_or_default(),
            e.end_date.unwrap_or_else(|| "now".into()),
            e.bullets.join(" ")
        ));
    }
    let skills = repo::list_skills(db).await?;
    if !skills.is_empty() {
        out.push_str(&format!("Skills: {}\n", skills.join(", ")));
    }
    Ok(out)
}

async fn posting_text(db: &Db, application_id: Option<i64>) -> Result<String> {
    let Some(id) = application_id else {
        return Ok(String::new());
    };
    let row = sqlx::query(
        "SELECT p.title, p.body, c.name FROM application a \
         LEFT JOIN posting p ON p.id = a.posting_id LEFT JOIN company c ON c.id = a.company_id \
         WHERE a.id = ?",
    )
    .bind(id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    Ok(row.map_or_else(String::new, |r| {
        format!(
            "{} at {}. {}",
            r.get::<Option<String>, _>("title").unwrap_or_default(),
            r.get::<Option<String>, _>("name").unwrap_or_default(),
            clip(
                &r.get::<Option<String>, _>("body").unwrap_or_default(),
                3000
            )
        )
    }))
}

/// Draft a reply to message `id` and save it as an email document tied to the application. A
/// message that already has a draft keeps it, unless `again` asks for a fresh one.
pub async fn draft_reply(
    db: &Db,
    worker: &ModelWorker,
    id: i64,
    instructions: &str,
    again: bool,
) -> Result<ReplyDraft> {
    let m = get_message(db, id).await?;
    if let (Some(doc), false) = (m.draft_document_id, again) {
        return Ok(ReplyDraft {
            document_id: doc,
            flags: vec![],
        });
    }
    let profile = profile_text(db).await?;
    let posting = posting_text(db, m.application_id).await?;
    let extra = if instructions.trim().is_empty() {
        String::new()
    } else {
        format!("- Also: {}", instructions.trim())
    };
    let from = m.from_addr.clone().unwrap_or_default();
    let vars = [
        ("profile", profile.as_str()),
        ("posting", posting.as_str()),
        ("from", from.as_str()),
        ("subject", m.subject.as_str()),
        ("body", clip(&m.body, 4000)),
        ("instructions", extra.as_str()),
    ];
    let out = run::<Answer>(worker, &DRAFT_REPLY, &vars, Priority::Background).await?;
    let flags = check(&out.value.reply, &[&profile, &posting, &m.body, &m.subject]);
    let created_with = format!("{}@{}", out.task_id, out.version);
    let title = format!("Reply: {}", m.subject);
    let doc = match m.draft_document_id {
        Some(doc) => {
            store::add_version(db, doc, &out.value.reply, "drafted", Some(&created_with)).await?;
            doc
        }
        None => {
            let doc = store::create(
                db,
                DocKind::Email,
                &title,
                m.application_id,
                None,
                &out.value.reply,
                "drafted",
            )
            .await?;
            set_draft(db, id, doc).await?;
            doc
        }
    };
    Ok(ReplyDraft {
        document_id: doc,
        flags,
    })
}
