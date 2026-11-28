//! Tie an employer email to the application it is about. Wrong links are worse than no link, so
//! when the evidence is ambiguous the mail stays unlinked and the user picks in the Inbox.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use sqlx::Row;

use crate::parse::ParsedMail;
use crate::store::host_of;

fn db_err(e: impl std::fmt::Display) -> Error {
    Error::Database(e.to_string())
}

struct Open {
    id: i64,
    title: String,
}

/// Applications of one company that are still in play, newest first.
async fn open_for_company(db: &Db, company_id: i64) -> Result<Vec<Open>> {
    let rows = sqlx::query(
        "SELECT a.id, COALESCE(p.title, '') AS title FROM application a \
         LEFT JOIN posting p ON p.id = a.posting_id \
         WHERE a.company_id = ? AND a.review <> 'dismissed' \
           AND a.status NOT IN ('accepted','rejected','declined','withdrawn') \
         ORDER BY a.id DESC",
    )
    .bind(company_id)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    Ok(rows
        .iter()
        .map(|r| Open {
            id: r.get("id"),
            title: r.get("title"),
        })
        .collect())
}

fn pick(open: &[Open], text: &str) -> Option<i64> {
    match open {
        [] => None,
        [one] => Some(one.id),
        many => {
            let text = text.to_ascii_lowercase();
            let hits: Vec<&Open> = many
                .iter()
                .filter(|o| !o.title.is_empty() && text.contains(&o.title.to_ascii_lowercase()))
                .collect();
            (hits.len() == 1).then(|| hits[0].id)
        }
    }
}

/// Companies whose mail domain or website matches the sender, or whose name (or an alias) is
/// named in the subject or by the labeller.
async fn companies_for(db: &Db, mail: &ParsedMail, named: Option<&str>) -> Result<Vec<i64>> {
    let rows = sqlx::query("SELECT id, name, aliases, website, email_domains FROM company")
        .fetch_all(db.pool())
        .await
        .map_err(db_err)?;
    let domains: Vec<String> = [mail.from_domain(), mail.reply_to_domain()]
        .into_iter()
        .flatten()
        .collect();
    let subject = mail.subject.to_ascii_lowercase();
    let named = named.map(str::to_ascii_lowercase);
    let mut by_domain = vec![];
    let mut by_name = vec![];
    for r in &rows {
        let id: i64 = r.get("id");
        let mut theirs: Vec<String> =
            serde_json::from_str::<Vec<String>>(&r.get::<String, _>("email_domains"))
                .unwrap_or_default()
                .into_iter()
                .map(|d| d.to_ascii_lowercase())
                .collect();
        if let Some(h) = r
            .get::<Option<String>, _>("website")
            .and_then(|w| host_of(&w))
        {
            theirs.push(h);
        }
        if domains.iter().any(|d| {
            theirs
                .iter()
                .any(|t| d == t || d.ends_with(&format!(".{t}")))
        }) {
            by_domain.push(id);
            continue;
        }
        let mut names = vec![r.get::<String, _>("name").to_ascii_lowercase()];
        names.extend(
            serde_json::from_str::<Vec<String>>(&r.get::<String, _>("aliases"))
                .unwrap_or_default()
                .into_iter()
                .map(|a| a.to_ascii_lowercase()),
        );
        if names.iter().any(|n| {
            n.len() >= 3 && (subject.contains(n.as_str()) || named.as_deref() == Some(n.as_str()))
        }) {
            by_name.push(id);
        }
    }
    Ok(if by_domain.is_empty() {
        by_name
    } else {
        by_domain
    })
}

/// The application this mail is about, if the evidence points to exactly one.
pub async fn find_application(
    db: &Db,
    mail: &ParsedMail,
    named_company: Option<&str>,
) -> Result<Option<i64>> {
    // 1. A reply in a thread we already tied to an application.
    for r in mail.references.iter().rev() {
        let hit: Option<i64> = sqlx::query_scalar(
            "SELECT application_id FROM email_message WHERE message_id = ? AND application_id IS NOT NULL LIMIT 1",
        )
        .bind(r)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?;
        if hit.is_some() {
            return Ok(hit);
        }
    }
    // 2. The sender's company, narrowed by job title when it has several open applications.
    let text = format!("{} {}", mail.subject, crate::label::clip(&mail.body, 2000));
    let companies = companies_for(db, mail, named_company).await?;
    let mut found = vec![];
    for c in companies {
        found.extend(open_for_company(db, c).await?);
    }
    Ok(pick(&found, &text))
}
