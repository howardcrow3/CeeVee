//! Bring an application list from somewhere else (Simplify's tracker export, a spreadsheet, any
//! job tracker) into CeeVee from a CSV. Columns are found by name, so the order and the extra
//! columns do not matter, and a status is read from plain words ("Interviewing", "Not selected").

use ceevee_core::{Error, Result};
use ceevee_db::Db;

use crate::tracking::{self, Status};

/// One row of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub company: String,
    pub title: String,
    pub status: Status,
    pub url: Option<String>,
    pub location: Option<String>,
}

/// What an import did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub added: usize,
    /// Already tracked, so left alone.
    pub existing: usize,
    /// Rows with no company or no job title.
    pub skipped: usize,
}

/// The status a tracker's own word stands for. Anything unclear counts as applied.
pub fn read_status(word: &str) -> Status {
    let w = word.trim().to_lowercase();
    let has = |s: &str| w.contains(s);
    if has("declin") && has("offer") {
        Status::Declined
    } else if has("withdr") {
        Status::Withdrawn
    } else if has("reject")
        || has("not selected")
        || has("no longer")
        || has("unsuccessful")
        || has("not moving")
        || has("denied")
    {
        Status::Rejected
    } else if has("accept") || has("hired") {
        Status::Accepted
    } else if has("offer") {
        Status::Offer
    } else if has("interview") || has("onsite") || has("on-site") || has("final round") {
        Status::Interview
    } else if has("screen") || has("recruiter") || has("in review") || has("assessment") {
        Status::Screening
    } else if has("wishlist")
        || has("saved")
        || has("bookmark")
        || has("draft")
        || has("to apply")
        || has("interested")
    {
        Status::Draft
    } else {
        Status::Sent
    }
}

fn col(h: &csv::StringRecord, names: &[&str]) -> Option<usize> {
    h.iter()
        .position(|c| names.iter().any(|n| c.trim().eq_ignore_ascii_case(n)))
}

/// Read the rows of an application list. Rows with no company or no title come back with empty
/// fields so the caller can count them as skipped.
pub fn parse_csv(text: &str) -> Result<Vec<Row>> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.trim_start_matches('\u{feff}').as_bytes());
    let h = rdr
        .headers()
        .map_err(|e| Error::Invalid(e.to_string()))?
        .clone();
    let company = col(&h, &["company", "company name", "employer", "organization"]);
    let title = col(
        &h,
        &[
            "job title",
            "title",
            "position",
            "role",
            "job",
            "position title",
        ],
    );
    if company.is_none() || title.is_none() {
        return Err(Error::Invalid(
            "this file needs a Company column and a Job Title column".into(),
        ));
    }
    let status = col(
        &h,
        &["status", "stage", "application status", "state", "progress"],
    );
    let url = col(
        &h,
        &[
            "url",
            "job url",
            "link",
            "job link",
            "posting url",
            "application url",
            "job posting",
        ],
    );
    let location = col(&h, &["location", "job location", "city"]);
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| Error::Invalid(e.to_string()))?;
        let get = |i: Option<usize>| {
            i.and_then(|i| rec.get(i))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        out.push(Row {
            company: get(company).unwrap_or_default(),
            title: get(title).unwrap_or_default(),
            status: get(status).map_or(Status::Sent, |s| read_status(&s)),
            url: get(url),
            location: get(location),
        });
    }
    Ok(out)
}

/// Track every row as an application. A job that is already tracked is left alone, so importing
/// the same file twice adds nothing the second time.
pub async fn import(db: &Db, rows: &[Row], now: &str) -> Result<Summary> {
    let mut known: std::collections::HashSet<i64> =
        tracking::list(db, &tracking::Filter::default())
            .await?
            .into_iter()
            .map(|a| a.id)
            .collect();
    let mut s = Summary::default();
    for r in rows {
        if r.company.trim().is_empty() || r.title.trim().is_empty() {
            s.skipped += 1;
            continue;
        }
        let app = tracking::add_manual_with(
            db,
            &r.company,
            &r.title,
            r.status,
            now,
            r.url.as_deref(),
            r.location.as_deref(),
        )
        .await?;
        if known.insert(app.id) {
            s.added += 1;
        } else {
            s.existing += 1;
        }
    }
    Ok(s)
}

#[cfg(test)]
mod tests;
