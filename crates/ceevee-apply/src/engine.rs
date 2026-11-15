//! The auto-fill engine. It drives a [`BrowserDriver`] through an application form: opens the
//! page (after the never-list check), reads the fields, fills the ones it can answer from the
//! person's profile and saved answers, and stops. It never clicks a submit control; submitting is
//! a separate, policy-gated step.
//!
//! The driver is a trait so the engine is tested against a scripted page, and the real browser
//! (Chrome or Edge over CDP) plugs in behind it.

use std::collections::BTreeMap;

use ceevee_core::{Error, Result};

use crate::ats::{detect, Ats};
use crate::fields::{classify_field, FieldKind};
use crate::guard::check_navigation;

/// What kind of control a field is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Text,
    Textarea,
    Select(Vec<String>),
    Checkbox,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormField {
    /// A handle the driver understands (a selector or an element id).
    pub id: String,
    pub label: String,
    pub input: Input,
    pub required: bool,
}

/// The slice of a browser the engine needs.
#[allow(async_fn_in_trait)]
pub trait BrowserDriver {
    async fn goto(&mut self, url: &str) -> Result<()>;
    async fn current_url(&mut self) -> Result<String>;
    /// Visible text of the page, used to spot CAPTCHA and sign-up pages.
    async fn page_text(&mut self) -> Result<String>;
    async fn fields(&mut self) -> Result<Vec<FormField>>;
    async fn fill(&mut self, field_id: &str, value: &str) -> Result<()>;
    async fn upload(&mut self, field_id: &str, path: &str) -> Result<()>;
}

/// Everything the engine may put into a form. Sensitive questions are answered only from `saved`.
#[derive(Debug, Clone, Default)]
pub struct Answers {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub city: String,
    pub linkedin: String,
    pub website: String,
    pub resume_pdf: Option<String>,
    pub cover_letter_pdf: Option<String>,
    /// Answers the person saved on purpose, keyed by a lower-case phrase from the question, for
    /// example "authorized to work" or "sponsorship" or "salary".
    pub saved: BTreeMap<String, String>,
}

impl Answers {
    /// Build the answers from a saved profile. `links` are the profile's web addresses; the
    /// LinkedIn one and the first other one are picked by their address. `facts` are the saved
    /// question answers; only the ones the person saved are ever used for sensitive questions.
    pub fn from_parts(
        full_name: &str,
        email: Option<&str>,
        phone: Option<&str>,
        location: Option<&str>,
        links: &[String],
        facts: &[(String, String)],
    ) -> Self {
        let full_name = full_name.trim();
        let (first, last) = match full_name.split_once(char::is_whitespace) {
            Some((f, l)) => (f.trim(), l.trim()),
            None => (full_name, ""),
        };
        let linkedin = links.iter().find(|l| l.contains("linkedin.com"));
        let website = links.iter().find(|l| !l.contains("linkedin.com"));
        // "Boston, MA" fills a city field with "Boston".
        let city = location
            .map(|l| l.split(',').next().unwrap_or(l).trim())
            .unwrap_or("");
        Self {
            first_name: first.to_string(),
            last_name: last.to_string(),
            email: email.unwrap_or("").trim().to_string(),
            phone: phone.unwrap_or("").trim().to_string(),
            city: city.to_string(),
            linkedin: linkedin.cloned().unwrap_or_default(),
            website: website.cloned().unwrap_or_default(),
            resume_pdf: None,
            cover_letter_pdf: None,
            saved: facts
                .iter()
                .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.clone()))
                .collect(),
        }
    }

    fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }

    /// A saved answer whose key appears in the question.
    pub fn saved_for(&self, label: &str) -> Option<&str> {
        let l = label.to_ascii_lowercase();
        self.saved
            .iter()
            .find(|(k, _)| !k.is_empty() && l.contains(k.as_str()))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filled {
    pub label: String,
    pub value: String,
    pub kind: FieldKind,
    /// Sensitive and salary answers are always shown to the person in the review list.
    pub review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub label: String,
    pub required: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillReport {
    pub ats: Ats,
    pub url: String,
    pub filled: Vec<Filled>,
    pub skipped: Vec<Skipped>,
}

impl FillReport {
    /// Required fields the engine could not answer: the person has to do these.
    pub fn missing_required(&self) -> Vec<&str> {
        self.skipped
            .iter()
            .filter(|s| s.required)
            .map(|s| s.label.as_str())
            .collect()
    }
}

fn refused(r: crate::guard::Refusal) -> Error {
    Error::Invalid(r.reason().to_string())
}

/// Open the posting and fill what can be answered. Stops before any submit control.
pub async fn fill_application<D: BrowserDriver>(
    driver: &mut D,
    url: &str,
    answers: &Answers,
) -> Result<FillReport> {
    check_navigation(url, None).map_err(refused)?;
    driver.goto(url).await?;
    let landed = driver.current_url().await?;
    let text = driver.page_text().await?;
    check_navigation(&landed, Some(&text)).map_err(refused)?;

    let mut report = FillReport {
        ats: detect(&landed),
        url: landed,
        filled: Vec::new(),
        skipped: Vec::new(),
    };
    for field in driver.fields().await? {
        let kind = classify_field(&field.label);
        let skip = |reason: &str| Skipped {
            label: field.label.clone(),
            required: field.required,
            reason: reason.to_string(),
        };
        // Uploads.
        if field.input == Input::File {
            let path = match kind {
                FieldKind::ResumeUpload => answers.resume_pdf.as_deref(),
                FieldKind::CoverLetter => answers.cover_letter_pdf.as_deref(),
                _ => None,
            };
            match path {
                Some(p) => {
                    driver.upload(&field.id, p).await?;
                    report.filled.push(Filled {
                        label: field.label.clone(),
                        value: p.to_string(),
                        kind,
                        review: false,
                    });
                }
                None => report.skipped.push(skip("no document to upload")),
            }
            continue;
        }
        let (value, review) = match kind {
            FieldKind::FirstName => (non_empty(&answers.first_name), false),
            FieldKind::LastName => (non_empty(&answers.last_name), false),
            FieldKind::FullName => (non_empty(&answers.full_name()), false),
            FieldKind::Email => (non_empty(&answers.email), false),
            FieldKind::Phone => (non_empty(&answers.phone), false),
            FieldKind::City => (non_empty(&answers.city), false),
            FieldKind::LinkedIn => (non_empty(&answers.linkedin), false),
            FieldKind::Website => (non_empty(&answers.website), false),
            FieldKind::Sensitive | FieldKind::Salary => {
                (answers.saved_for(&field.label).map(str::to_string), true)
            }
            FieldKind::ResumeUpload | FieldKind::CoverLetter | FieldKind::Unknown => (None, false),
        };
        let Some(mut value) = value else {
            let why = match kind {
                FieldKind::Sensitive | FieldKind::Salary => "needs an answer you saved",
                FieldKind::Unknown => "not recognised",
                _ => "nothing saved for this",
            };
            report.skipped.push(skip(why));
            continue;
        };
        // A dropdown only accepts one of its options.
        if let Input::Select(options) = &field.input {
            match pick_option(options, &value) {
                Some(o) => value = o,
                None => {
                    report.skipped.push(skip("no matching choice in the list"));
                    continue;
                }
            }
        }
        driver.fill(&field.id, &value).await?;
        report.filled.push(Filled {
            label: field.label.clone(),
            value,
            kind,
            review,
        });
    }
    Ok(report)
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// The option that equals the value, or failing that contains it, ignoring case.
fn pick_option(options: &[String], value: &str) -> Option<String> {
    let v = value.trim().to_ascii_lowercase();
    options
        .iter()
        .find(|o| o.trim().to_ascii_lowercase() == v)
        .or_else(|| options.iter().find(|o| o.to_ascii_lowercase().contains(&v)))
        .cloned()
}

#[cfg(test)]
mod tests;
