//! Work out what a form field is asking for from its label, so the filler can pull the answer
//! from the profile. Sensitive questions are flagged: they are only ever answered from the
//! person's saved answers, never guessed by a model.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    FirstName,
    LastName,
    FullName,
    Email,
    Phone,
    City,
    LinkedIn,
    Website,
    ResumeUpload,
    CoverLetter,
    /// Work authorisation, sponsorship, veteran, disability, gender, race and similar.
    Sensitive,
    /// Salary or compensation expectations.
    Salary,
    Unknown,
}

fn has(s: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| s.contains(n))
}

pub fn classify_field(label: &str) -> FieldKind {
    let l = label.to_ascii_lowercase();
    let l = l.trim().trim_end_matches('*').trim();
    if has(
        l,
        &[
            "authorized to work",
            "authorised to work",
            "sponsorship",
            "visa",
            "veteran",
            "disability",
            "gender",
            "race",
            "ethnic",
            "pronoun",
            "sexual orientation",
            "criminal",
            "convicted",
            "citizenship",
        ],
    ) {
        return FieldKind::Sensitive;
    }
    if has(l, &["salary", "compensation", "pay expectation"]) {
        return FieldKind::Salary;
    }
    if has(l, &["cover letter"]) {
        return FieldKind::CoverLetter;
    }
    if has(l, &["resume", "résumé", "cv"]) && !has(l, &["curriculum"])
        || has(l, &["curriculum vitae"])
    {
        return FieldKind::ResumeUpload;
    }
    if has(l, &["linkedin"]) {
        return FieldKind::LinkedIn;
    }
    if has(l, &["website", "portfolio", "github"]) {
        return FieldKind::Website;
    }
    if has(l, &["first name", "given name"]) {
        return FieldKind::FirstName;
    }
    if has(l, &["last name", "surname", "family name"]) {
        return FieldKind::LastName;
    }
    if l == "name" || l == "full name" || l == "your name" {
        return FieldKind::FullName;
    }
    if has(l, &["email", "e-mail"]) {
        return FieldKind::Email;
    }
    if has(l, &["phone", "mobile", "telephone"]) {
        return FieldKind::Phone;
    }
    if has(l, &["city", "location", "where are you based"]) {
        return FieldKind::City;
    }
    FieldKind::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use FieldKind::*;

    #[test]
    fn common_labels() {
        for (label, want) in [
            ("First Name *", FirstName),
            ("Last name", LastName),
            ("Full name", FullName),
            ("Email address", Email),
            ("Phone number", Phone),
            ("LinkedIn Profile", LinkedIn),
            ("Personal website or portfolio", Website),
            ("Resume/CV", ResumeUpload),
            ("Cover letter", CoverLetter),
            ("Current city", City),
            ("Desired salary", Salary),
            ("Favourite colour", Unknown),
        ] {
            assert_eq!(classify_field(label), want, "{label}");
        }
    }

    #[test]
    fn sensitive_questions_are_never_treated_as_ordinary_fields() {
        for label in [
            "Are you legally authorized to work in the United States?",
            "Will you now or in the future require visa sponsorship?",
            "Veteran status",
            "Gender identity",
            "Please select your race/ethnicity",
        ] {
            assert_eq!(classify_field(label), Sensitive, "{label}");
        }
    }
}
