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
    /// The cover letter attached, if one was saved for this job.
    pub cover_letter_title: Option<String>,
    /// For the application record: the posting, its application if there is one, and the exact
    /// document versions that were attached.
    pub posting_id: i64,
    pub application_id: Option<i64>,
    pub resume_version_id: Option<i64>,
    pub cover_letter_version_id: Option<i64>,
}

/// Gather the inputs for applying to a posting. The resume PDF is written into `pdf_dir`.
/// A resume and a cover letter saved for this posting's application are used first. Otherwise the
/// newest tailored resume, then the newest base resume, are used, and no cover letter is attached.
pub async fn gather(db: &Db, posting_id: i64, pdf_dir: &Path, paper: Paper) -> Result<Inputs> {
    let posting = postings::get(db, posting_id).await?;
    let mut answers = answers_from_profile(db).await?;

    let application =
        ceevee_search::tracking::list(db, &ceevee_search::tracking::Filter::default())
            .await?
            .into_iter()
            .find(|a| a.posting_id == Some(posting_id))
            .map(|a| a.id);

    let mut resume_title = None;
    let mut resume_version_id = None;
    let mut candidates = Vec::new();
    if let Some(app) = application {
        candidates.push((DocKind::TailoredResume, Some(app)));
    }
    candidates.push((DocKind::TailoredResume, None));
    candidates.push((DocKind::BaseResume, None));
    for (kind, app) in candidates {
        if let Some(doc) = store::list(db, Some(kind), app).await?.into_iter().next() {
            let cur = store::current(db, doc.id).await?;
            let bytes = pdf_for(doc.kind, &cur.content, paper)?;
            std::fs::create_dir_all(pdf_dir)?;
            let path = pdf_dir.join(format!("{}.pdf", file_stem(&doc.title)));
            std::fs::write(&path, bytes)?;
            answers.resume_pdf = Some(path.display().to_string());
            resume_title = Some(doc.title);
            resume_version_id = Some(cur.id);
            break;
        }
    }
    let mut cover_letter_title = None;
    let mut cover_letter_version_id = None;
    if let Some(app) = application {
        if let Some(doc) = store::list(db, Some(DocKind::CoverLetter), Some(app))
            .await?
            .into_iter()
            .next()
        {
            let cur = store::current(db, doc.id).await?;
            let bytes = pdf_for(doc.kind, &cur.content, paper)?;
            std::fs::create_dir_all(pdf_dir)?;
            let path = pdf_dir.join(format!("{}.pdf", file_stem(&doc.title)));
            std::fs::write(&path, bytes)?;
            answers.cover_letter_pdf = Some(path.display().to_string());
            cover_letter_title = Some(doc.title);
            cover_letter_version_id = Some(cur.id);
        }
    }
    Ok(Inputs {
        url: posting.url,
        company_and_title: posting.title,
        answers,
        resume_title,
        cover_letter_title,
        posting_id,
        application_id: application,
        resume_version_id,
        cover_letter_version_id,
    })
}

/// The answers the person's profile gives: name, contact details, links and the answers they
/// saved on purpose for sensitive questions.
async fn answers_from_profile(db: &Db) -> Result<Answers> {
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
    Ok(Answers::from_parts(
        &basics.full_name,
        basics.email.as_deref(),
        basics.phone.as_deref(),
        basics.location.as_deref(),
        &links,
        &facts,
    ))
}

/// Answers for a page that is not one of the saved jobs, such as one the person found in their
/// own browser: the profile and the newest base resume as a PDF in `pdf_dir`.
pub async fn gather_general(db: &Db, pdf_dir: &Path, paper: Paper) -> Result<Answers> {
    let mut answers = answers_from_profile(db).await?;
    if let Some(doc) = store::list(db, Some(DocKind::BaseResume), None)
        .await?
        .into_iter()
        .next()
    {
        let cur = store::current(db, doc.id).await?;
        let bytes = pdf_for(doc.kind, &cur.content, paper)?;
        std::fs::create_dir_all(pdf_dir)?;
        let path = pdf_dir.join(format!("{}.pdf", file_stem(&doc.title)));
        std::fs::write(&path, bytes)?;
        answers.resume_pdf = Some(path.display().to_string());
    }
    Ok(answers)
}

#[cfg(test)]
mod tests;
