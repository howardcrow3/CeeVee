//! Interview practice from the real job posting and the person's own profile, with the local
//! model. Questions and feedback are advice only. Each answered question can be kept as a note
//! with the application, so it is there again before the real interview.

use ceevee_ai::task::{run, INTERVIEW_FEEDBACK, INTERVIEW_QUESTIONS};
use ceevee_ai::{ModelWorker, Priority};
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde::Deserialize;
use sqlx::Row;

#[derive(Deserialize)]
struct Questions {
    questions: Vec<String>,
}

#[derive(Deserialize)]
struct Feedback {
    feedback: String,
}

/// Five likely questions for this job. Blank or repeated questions are dropped.
pub async fn questions(
    worker: &ModelWorker,
    profile_text: &str,
    posting_text: &str,
) -> Result<Vec<String>> {
    let vars = [("profile", profile_text), ("posting", posting_text)];
    let out = run::<Questions>(worker, &INTERVIEW_QUESTIONS, &vars, Priority::Interactive).await?;
    Ok(clean(out.value.questions))
}

fn clean(list: Vec<String>) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for q in list {
        let q = q
            .trim()
            .trim_start_matches(|c: char| c.is_ascii_digit() || ".) -".contains(c));
        let q = q.trim();
        let key = q.to_lowercase();
        if q.is_empty() || seen.contains(&key) {
            continue;
        }
        seen.push(key);
        out.push(q.to_string());
    }
    out
}

/// Coaching feedback on one practice answer.
pub async fn feedback(
    worker: &ModelWorker,
    question: &str,
    answer: &str,
    profile_text: &str,
    posting_text: &str,
) -> Result<String> {
    if answer.trim().split_whitespace().count() < 5 {
        return Err(Error::Invalid(
            "Write a few sentences first, then ask for feedback.".into(),
        ));
    }
    let vars = [
        ("question", question),
        ("answer", answer),
        ("profile", profile_text),
        ("posting", posting_text),
    ];
    let out = run::<Feedback>(worker, &INTERVIEW_FEEDBACK, &vars, Priority::Interactive).await?;
    Ok(out.value.feedback.trim().to_string())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub id: i64,
    pub question: String,
    pub answer: String,
    pub feedback: String,
    pub created_at: String,
}

/// Keep an answered question with the application.
pub async fn save_note(
    db: &Db,
    application_id: i64,
    question: &str,
    answer: &str,
    feedback: &str,
) -> Result<i64> {
    if question.trim().is_empty() || answer.trim().is_empty() {
        return Err(Error::Invalid(
            "a note needs a question and an answer".into(),
        ));
    }
    let id = sqlx::query_scalar(
        "INSERT INTO practice_note (application_id, question, answer, feedback) \
         VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(application_id)
    .bind(question.trim())
    .bind(answer.trim())
    .bind(feedback.trim())
    .fetch_one(db.pool())
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    Ok(id)
}

/// The notes kept with an application, oldest first.
pub async fn notes(db: &Db, application_id: i64) -> Result<Vec<Note>> {
    let rows = sqlx::query(
        "SELECT id, question, answer, feedback, created_at FROM practice_note \
         WHERE application_id = ? ORDER BY id",
    )
    .bind(application_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    Ok(rows
        .iter()
        .map(|r| Note {
            id: r.get("id"),
            question: r.get("question"),
            answer: r.get("answer"),
            feedback: r.get("feedback"),
            created_at: r.get("created_at"),
        })
        .collect())
}

pub async fn delete_note(db: &Db, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM practice_note WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests;
