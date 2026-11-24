//! Spot mail that is about a job search even when the sender is nobody CeeVee knows: a
//! recruiter's personal address, a staffing agency, a company that was never added. The score
//! comes from plain phrases in the subject, the sender and the start of the body, so it works
//! with no model. With a model, the score only decides whether the model is asked.

use crate::parse::ParsedMail;

/// A score this high is worth asking the model about.
pub const ASK_MODEL: i32 = 3;
/// A score this high is accepted on the rules alone.
pub const ACCEPT_ALONE: i32 = 5;

/// Phrases that nearly always mean a job-search message. Two points each.
const STRONG: &[&str] = &[
    "your application",
    "application for",
    "applied for",
    "applied to",
    "thank you for applying",
    "thanks for applying",
    "interview",
    "phone screen",
    "recruiter",
    "recruiting",
    "talent acquisition",
    "sourcer",
    "staffing",
    "your candidacy",
    "the candidate",
    "job opportunity",
    "career opportunity",
    "open position",
    "open role",
    "job opening",
    "we are hiring",
    "we're hiring",
    "hiring for",
    "hiring manager",
    "offer of employment",
    "job offer",
    "offer letter",
    "came across your profile",
    "found your profile",
    "your resume",
    "your cv",
    "your background",
    "not be moving forward",
    "pursue other candidates",
    "next steps in the process",
    "hiring process",
];

/// Phrases that point to a job search but also turn up elsewhere. One point each.
const WEAK: &[&str] = &[
    "position",
    "role",
    "opportunity",
    "hiring",
    "candidate",
    "salary",
    "compensation",
    "your experience",
    "career",
    "assessment",
    "coding challenge",
    "job",
    "apply",
    "application",
    "linkedin",
    "resume",
    "interested in",
    "open to",
];

/// Marketing and receipts. Three points off each.
const NOISE: &[&str] = &[
    "% off",
    "sale ends",
    "free trial",
    "your order",
    "order confirmation",
    "your receipt",
    "invoice",
    "view in browser",
    "view this email in your browser",
    "shipping confirmation",
    "your subscription",
    "weekly digest",
    "newsletter",
    "limited time",
];

/// Parts of a sender name or address that suggest hiring staff. Two points.
const SENDER: &[&str] = &[
    "recruit", "talent", "careers", "hiring", "staffing", "sourcing",
];

/// How much of the body is read for phrases.
const SCAN: usize = 3000;

fn has(text: &str, phrase: &str) -> bool {
    text.contains(phrase)
}

pub fn score(mail: &ParsedMail) -> i32 {
    let body = crate::label::clip(&mail.body, SCAN);
    let text = format!("{}\n{}", mail.subject, body).to_ascii_lowercase();
    let mut points = 0;
    for p in STRONG {
        if has(&text, p) {
            points += 2;
        }
    }
    for p in WEAK {
        if has(&text, p) {
            points += 1;
        }
    }
    // The subject counts a second time: a person who writes "Interview" there means it.
    let subject = mail.subject.to_ascii_lowercase();
    if STRONG.iter().any(|p| has(&subject, p)) {
        points += 2;
    }
    let sender = format!(
        "{} {}",
        mail.from_name.as_deref().unwrap_or(""),
        mail.from_address.as_deref().unwrap_or("")
    )
    .to_ascii_lowercase();
    if SENDER.iter().any(|p| has(&sender, p)) {
        points += 2;
    }
    for p in NOISE {
        if has(&text, p) {
            points -= 3;
        }
    }
    points
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing to do with a job search.
    No,
    /// Could be; only a model can say.
    Maybe,
    /// Clearly about a job search.
    Yes,
}

pub fn verdict(mail: &ParsedMail) -> Verdict {
    match score(mail) {
        s if s >= ACCEPT_ALONE => Verdict::Yes,
        s if s >= ASK_MODEL => Verdict::Maybe,
        _ => Verdict::No,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mail(from_name: &str, from: &str, subject: &str, body: &str) -> ParsedMail {
        ParsedMail {
            message_id: None,
            references: vec![],
            from_name: Some(from_name.into()),
            from_address: Some(from.into()),
            reply_to: None,
            subject: subject.into(),
            sent_at: None,
            body: body.into(),
        }
    }

    #[test]
    fn a_cold_recruiter_message_is_a_yes() {
        let m = mail(
            "Sam Lee",
            "sam@orbitstaffing.com",
            "Senior Data Engineer opportunity",
            "Hi Maria, I'm a recruiter at Orbit Staffing and came across your profile. We have an open position for a Senior Data Engineer. Are you open to a quick call?",
        );
        assert_eq!(verdict(&m), Verdict::Yes);
    }

    #[test]
    fn everyday_mail_is_a_no() {
        for (s, b) in [
            ("Dinner?", "Are you free Friday?"),
            (
                "Your order has shipped",
                "Your order confirmation and invoice are attached.",
            ),
            (
                "Team lunch",
                "We are getting lunch at noon. Which role do you want to play in the game?",
            ),
        ] {
            assert_eq!(verdict(&mail("A", "a@gmail.com", s, b)), Verdict::No, "{s}");
        }
    }

    #[test]
    fn a_thin_hint_is_left_to_the_model() {
        let m = mail(
            "Pat",
            "pat@example.com",
            "Quick question",
            "The position is open for this role. Are you interested in the opportunity?",
        );
        assert_eq!(verdict(&m), Verdict::Maybe);
    }

    #[test]
    fn marketing_that_mentions_jobs_is_pushed_down() {
        let m = mail(
            "Deals",
            "news@shop.example",
            "Job hunting gear: 40% off",
            "View in browser. Newsletter: limited time sale on resume paper and interview shirts!",
        );
        assert_eq!(verdict(&m), Verdict::No);
    }

    #[test]
    fn an_ats_style_status_mail_is_a_yes() {
        let m = mail(
            "Orbit Software",
            "no-reply@somewhere.example",
            "Your application to Orbit Software",
            "Thank you for interviewing with us. We have decided to pursue other candidates for this position.",
        );
        assert_eq!(verdict(&m), Verdict::Yes);
    }
}
