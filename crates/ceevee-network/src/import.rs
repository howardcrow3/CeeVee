//! Bring people in from a vCard file (`.vcf`, from Apple Contacts, Outlook, Google Contacts and
//! most phones) or a contacts CSV (Google Contacts, Outlook, or a simple name and email sheet).
//! Anyone who matches a person already saved is filled in, not duplicated.

use crate::people::{self, NewPerson};
use ceevee_core::{Error, Result};
use ceevee_db::Db;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub added: usize,
    /// Already saved: blanks were filled in.
    pub matched: usize,
    /// Rows with no name that were skipped.
    pub skipped: usize,
}

fn unescape(v: &str) -> String {
    v.replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
        .trim()
        .to_string()
}

/// Parse vCard text (versions 2.1, 3.0 and 4.0 share the lines used here).
pub fn parse_vcards(text: &str) -> Vec<NewPerson> {
    // Unfold: a line starting with a space or tab continues the one before.
    let mut lines: Vec<String> = Vec::new();
    for raw in text.trim_start_matches('\u{feff}').lines() {
        if raw.starts_with(' ') || raw.starts_with('\t') {
            if let Some(last) = lines.last_mut() {
                last.push_str(raw.trim_start());
                continue;
            }
        }
        lines.push(raw.trim_end().to_string());
    }
    let mut out = Vec::new();
    let mut cur: Option<NewPerson> = None;
    let mut n_name = String::new();
    for line in lines {
        let upper = line.to_ascii_uppercase();
        if upper == "BEGIN:VCARD" {
            cur = Some(NewPerson::default());
            n_name.clear();
            continue;
        }
        if upper == "END:VCARD" {
            if let Some(mut p) = cur.take() {
                if p.full_name.is_empty() {
                    p.full_name = n_name.clone();
                }
                out.push(p);
            }
            continue;
        }
        let Some(p) = cur.as_mut() else { continue };
        let Some((head, value)) = line.split_once(':') else {
            continue;
        };
        let name = head.split(';').next().unwrap_or("").to_ascii_uppercase();
        // Apple prefixes properties with a group such as "item1."
        let name = name.rsplit('.').next().unwrap_or("").to_string();
        let value = unescape(value);
        if value.is_empty() {
            continue;
        }
        match name.as_str() {
            "FN" => p.full_name = value,
            "N" => {
                let parts: Vec<&str> = value.split(';').collect();
                let (last, first) = (
                    parts.first().copied().unwrap_or(""),
                    parts.get(1).copied().unwrap_or(""),
                );
                n_name = format!("{first} {last}").trim().to_string();
            }
            "EMAIL" if p.email.is_none() => p.email = Some(value),
            "TEL" if p.phone.is_none() => p.phone = Some(value),
            "ORG" if p.company.is_none() => {
                p.company = value
                    .split(';')
                    .next()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
            }
            "TITLE" if p.title.is_none() => p.title = Some(value),
            "URL" if p.profile_url.is_none() => p.profile_url = Some(value),
            "NOTE" if p.notes.is_none() => p.notes = Some(value),
            _ => {}
        }
    }
    out
}

fn col(headers: &csv::StringRecord, names: &[&str]) -> Option<usize> {
    headers
        .iter()
        .position(|h| names.iter().any(|n| h.trim().eq_ignore_ascii_case(n)))
}

/// Parse a contacts CSV. Columns are found by name, so order does not matter.
pub fn parse_csv(text: &str) -> Result<Vec<NewPerson>> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.trim_start_matches('\u{feff}').as_bytes());
    let h = rdr
        .headers()
        .map_err(|e| Error::Invalid(e.to_string()))?
        .clone();
    let name = col(&h, &["name", "full name"]);
    let first = col(&h, &["first name", "given name"]);
    let last = col(&h, &["last name", "family name", "surname"]);
    if name.is_none() && first.is_none() && last.is_none() {
        return Err(Error::Invalid(
            "this file has no Name or First Name column".into(),
        ));
    }
    let email = col(
        &h,
        &[
            "email",
            "email address",
            "e-mail",
            "e-mail address",
            "e-mail 1 - value",
        ],
    );
    let phone = col(
        &h,
        &["phone", "mobile phone", "phone 1 - value", "primary phone"],
    );
    let company = col(
        &h,
        &[
            "company",
            "organization",
            "organization name",
            "organization 1 - name",
        ],
    );
    let title = col(
        &h,
        &[
            "title",
            "position",
            "job title",
            "organization title",
            "organization 1 - title",
        ],
    );
    let url = col(&h, &["url", "linkedin", "profile url", "website 1 - value"]);
    let notes = col(&h, &["notes", "note"]);
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| Error::Invalid(e.to_string()))?;
        let get = |i: Option<usize>| {
            i.and_then(|i| rec.get(i))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        let full = get(name).unwrap_or_else(|| {
            format!(
                "{} {}",
                get(first).unwrap_or_default(),
                get(last).unwrap_or_default()
            )
            .trim()
            .to_string()
        });
        out.push(NewPerson {
            full_name: full,
            email: get(email),
            phone: get(phone),
            profile_url: get(url),
            notes: get(notes),
            company: get(company),
            title: get(title),
            ..NewPerson::default()
        });
    }
    Ok(out)
}

/// Save people. Someone who matches a saved person fills that person's blanks. `source` is
/// stored with new people, for example `vcard` or `csv`.
pub async fn save(db: &Db, people_in: &[NewPerson], source: &str) -> Result<Summary> {
    let mut s = Summary::default();
    for p in people_in {
        if p.full_name.trim().is_empty() {
            s.skipped += 1;
            continue;
        }
        match people::find_match(db, p).await? {
            Some(id) => {
                let have = people::get(db, id).await?;
                let mut merged = NewPerson {
                    full_name: have.full_name.clone(),
                    email: have.email.clone().or_else(|| p.email.clone()),
                    phone: have.phone.clone().or_else(|| p.phone.clone()),
                    profile_url: have.profile_url.clone().or_else(|| p.profile_url.clone()),
                    how_met: have.how_met.clone(),
                    notes: have.notes.clone().or_else(|| p.notes.clone()),
                    tags: have.tags.clone(),
                    company: None,
                    title: None,
                };
                // Only set a role when the saved person has none.
                if have.company.is_none() {
                    merged.company = p.company.clone();
                    merged.title = p.title.clone();
                }
                people::update(db, id, &merged).await?;
                s.matched += 1;
            }
            None => {
                people::add(db, p, source).await?;
                s.added += 1;
            }
        }
    }
    Ok(s)
}

#[cfg(test)]
mod tests;
