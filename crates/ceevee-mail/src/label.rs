//! Say what an employer email is. The model reads the email as data and can only choose a label
//! from a fixed list; code decides what to do with it. When no model is loaded, or it fails, plain
//! keyword rules give a label so the Inbox still works.

use ceevee_ai::task::{run, LABEL_EMAIL};
use ceevee_ai::{ModelWorker, Priority};
use ceevee_search::tracking::Status;
use serde::Deserialize;

use crate::parse::ParsedMail;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Interview,
    Assessment,
    Offer,
    Rejection,
    Question,
    Acknowledgement,
    Other,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Interview => "interview",
            Self::Assessment => "assessment",
            Self::Offer => "offer",
            Self::Rejection => "rejection",
            Self::Question => "question",
            Self::Acknowledgement => "acknowledgement",
            Self::Other => "other",
        }
    }

    /// The status this kind of mail points to, if any.
    pub fn status(self) -> Option<Status> {
        match self {
            Self::Interview => Some(Status::Interview),
            Self::Assessment => Some(Status::Screening),
            Self::Offer => Some(Status::Offer),
            Self::Rejection => Some(Status::Rejected),
            Self::Acknowledgement => Some(Status::Sent),
            Self::Question | Self::Other => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub kind: Kind,
    pub summary: String,
    pub needs_reply: bool,
    /// Interview start as the email wrote it.
    pub interview_at: Option<String>,
    pub company: Option<String>,
    /// `label_email@1`, or `rules@1` when the keyword rules decided.
    pub with: String,
}

#[derive(Deserialize)]
struct Answer {
    kind: Kind,
    summary: String,
    needs_reply: bool,
    interview_at: String,
    mentions_company: String,
}

fn non_empty(s: String) -> Option<String> {
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Longest part of a body shown to the model; the start of a mail holds the message.
const BODY_LIMIT: usize = 4000;

pub fn clip(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Keyword rules. Rejections are checked first because they often also say "interview".
pub fn by_rules(mail: &ParsedMail) -> Label {
    let text = format!("{} {}", mail.subject, clip(&mail.body, BODY_LIMIT)).to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| text.contains(w));
    let kind = if has(&[
        "unfortunately",
        "not be moving forward",
        "not moving forward",
        "will not be proceeding",
        "decided to pursue other candidates",
        "decided not to move forward",
        "position has been filled",
        "regret to inform",
    ]) {
        Kind::Rejection
    } else if has(&[
        "pleased to offer",
        "offer of employment",
        "extend an offer",
        "excited to offer",
        "offer letter",
    ]) {
        Kind::Offer
    } else if has(&[
        "schedule an interview",
        "schedule a call",
        "phone screen",
        "interview with",
        "available for an interview",
        "invite you to interview",
        "interview invitation",
        "your interview",
    ]) {
        Kind::Interview
    } else if has(&[
        "assessment",
        "coding challenge",
        "take-home",
        "take home",
        "skills test",
        "complete the following",
    ]) {
        Kind::Assessment
    } else if has(&[
        "thank you for applying",
        "thanks for applying",
        "we received your application",
        "application received",
        "we have received your application",
    ]) {
        Kind::Acknowledgement
    } else if text.contains('?') {
        Kind::Question
    } else {
        Kind::Other
    };
    let needs_reply = matches!(kind, Kind::Interview | Kind::Question | Kind::Offer);
    Label {
        kind,
        summary: mail.subject.chars().take(200).collect(),
        needs_reply,
        interview_at: None,
        company: None,
        with: "rules@1".into(),
    }
}

/// Label with the model when there is one, otherwise (or when it fails) with the keyword rules.
pub async fn label(worker: Option<&ModelWorker>, mail: &ParsedMail) -> Label {
    let Some(worker) = worker else {
        return by_rules(mail);
    };
    let from = mail
        .from_address
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let vars = [
        ("from", from.as_str()),
        ("subject", mail.subject.as_str()),
        ("body", clip(&mail.body, BODY_LIMIT)),
    ];
    match run::<Answer>(worker, &LABEL_EMAIL, &vars, Priority::Background).await {
        Ok(out) => Label {
            kind: out.value.kind,
            summary: out.value.summary,
            needs_reply: out.value.needs_reply,
            interview_at: non_empty(out.value.interview_at),
            company: non_empty(out.value.mentions_company),
            with: format!("{}@{}", out.task_id, out.version),
        },
        Err(_) => by_rules(mail),
    }
}

/// `YYYY-MM-DD HH:MM` or `YYYY-MM-DDTHH:MM` at the start of `s`, as `YYYY-MM-DDTHH:MM`.
pub fn parse_when(s: &str) -> Option<String> {
    let b = s.trim().as_bytes();
    if b.len() < 16 {
        return None;
    }
    let digit = |i: usize| b[i].is_ascii_digit();
    let ok = (0..4).all(digit)
        && b[4] == b'-'
        && (5..7).all(digit)
        && b[7] == b'-'
        && (8..10).all(digit)
        && (b[10] == b' ' || b[10] == b'T')
        && (11..13).all(digit)
        && b[13] == b':'
        && (14..16).all(digit);
    ok.then(|| {
        let t = std::str::from_utf8(&b[..16]).unwrap_or_default();
        format!("{}T{}", &t[..10], &t[11..16])
    })
}
