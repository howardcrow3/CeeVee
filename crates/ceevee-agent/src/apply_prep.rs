//! Everything the form filler needs before the browser opens: the posting's address, the person's
//! answers, and a PDF of their resume. Kept apart from the browser so it is tested without one.

use std::path::Path;

use ceevee_apply::engine::Answers;
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use ceevee_docs::export::{file_stem, pdf_for};
use ceevee_docs::render::Paper;
use ceevee_docs::store::{self, DocKind};
use ceevee_profile::repo;
use ceevee_search::postings;

/// What to open and what to put in it.
pub struct Inputs {
    pub url: String,
    pub company_and_title: String,
    pub answers: Answers,
    /// The resume used, so the person can see which one was attached.
    pub resume_title: Option<String>,
}

/// Gather the inputs for applying to a posting. The resume PDF is written into `pdf_dir`.
/// The newest tailored resume is used if there is one, else the newest base resume. A cover letter
/// is only attached when it belongs to an application, which is not the case yet.
pub async fn gather(db: &Db, posting_id: i64, pdf_dir: &Path, paper: Paper) -> Result<Inputs> {
    let posting = postings::get(db, posting_id).await?;
    let basics = repo::get_basics(db).await?;
    if basics.full_name.trim().is_empty() {
        return Err(Error::Invalid(
            "Add your name to your profile before applying.".into(),
        ));
    }
    let links = repo::get_links(db).await?;
    let facts: Vec<(String, String)> = repo::list_facts(db)
        .await?
        .into_iter()
        .map(|f| (f.question_key, f.answer))
        .collect();
    let mut answers = Answers::from_parts(
        &basics.full_name,
        basics.email.as_deref(),
        basics.phone.as_deref(),
        basics.location.as_deref(),
        &links,
        &facts,
    );

    let mut resume_title = None;
    for kind in [DocKind::TailoredResume, DocKind::BaseResume] {
        if let Some(doc) = store::list(db, Some(kind), None).await?.into_iter().next() {
            let cur = store::current(db, doc.id).await?;
            let bytes = pdf_for(doc.kind, &cur.content, paper)?;
            std::fs::create_dir_all(pdf_dir)?;
            let path = pdf_dir.join(format!("{}.pdf", file_stem(&doc.title)));
            std::fs::write(&path, bytes)?;
            answers.resume_pdf = Some(path.display().to_string());
            resume_title = Some(doc.title);
            break;
        }
    }
    Ok(Inputs {
        url: posting.url,
        company_and_title: posting.title,
        answers,
        resume_title,
    })
}

#[cfg(test)]
mod tests;
