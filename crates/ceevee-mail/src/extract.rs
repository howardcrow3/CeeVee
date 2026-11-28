//! Pull the company, job title and place out of an email with plain rules. This is the fallback
//! for when no model is loaded, and it fills in what a model leaves blank. It prefers returning
//! nothing to returning a guess: an auto-created job is shown to the user to confirm, but a wrong
//! company name would still be noise.

use crate::filter::KNOWN_ATS_DOMAINS;
use crate::parse::ParsedMail;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extracted {
    pub company: Option<String>,
    pub title: Option<String>,
    pub location: Option<String>,
}

/// Mailbox providers, whose domain says nothing about an employer.
const GENERIC_DOMAINS: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "outlook.com",
    "hotmail.com",
    "live.com",
    "msn.com",
    "yahoo.com",
    "icloud.com",
    "me.com",
    "aol.com",
    "proton.me",
    "protonmail.com",
    "comcast.net",
    "linkedin.com",
    "indeed.com",
    "ziprecruiter.com",
    "glassdoor.com",
];

const NOT_A_NAME: &[&str] = &[
    "you",
    "your",
    "us",
    "our",
    "we",
    "the",
    "a",
    "an",
    "this",
    "that",
    "them",
    "it",
    "me",
    "my",
    "team",
    "company",
    "role",
    "position",
    "job",
    "interview",
    "application",
    "hiring",
    "recruiting",
];

/// Words that end a company name or a job title when they follow it.
const COMPANY_STOPS: &[&str] = &[
    " for ",
    " as ",
    " is ",
    " are ",
    " and ",
    " has ",
    " was ",
    " on ",
    " in ",
    " - ",
    " \u{2013} ",
    " \u{2014} ",
    " we ",
    " to ",
    " that ",
    " who ",
    " which ",
    " where ",
    " team",
    " about ",
    " with ",
    " this ",
    " next ",
];
const TITLE_STOPS: &[&str] = &[
    " position",
    " role",
    " opening",
    " opportunity",
    " at ",
    " with ",
    " on ",
    " here",
    " in ",
    " - ",
    " \u{2013} ",
    " \u{2014} ",
    " is ",
    " has ",
    " and we ",
    " that ",
    " for ",
    " we ",
    " as ",
    " within ",
    " team",
    " starting ",
];

/// Cut a captured span at the first newline, sentence mark or stop word.
fn trim_span(rest: &str, stops: &[&str], keep_comma: bool) -> String {
    let mut s = rest.split('\n').next().unwrap_or("");
    let enders: &[char] = if keep_comma {
        &[
            '.', ';', ':', '!', '?', '(', ')', '|', '"', '\u{201c}', '\u{201d}',
        ]
    } else {
        &[
            '.', ',', ';', ':', '!', '?', '(', ')', '|', '"', '\u{201c}', '\u{201d}',
        ]
    };
    // A trailing "Inc." or "Ltd." belongs to the name; keep the word, drop the dot.
    let mut cut = s.len();
    for (i, c) in s.char_indices() {
        if enders.contains(&c) {
            cut = i;
            break;
        }
    }
    s = &s[..cut];
    let lower = s.to_ascii_lowercase();
    let mut end = s.len();
    for stop in stops {
        if let Some(i) = lower.find(stop) {
            end = end.min(i);
        }
    }
    s[..end].trim().trim_matches(['\'', '\u{2019}']).to_string()
}

fn valid(name: &str, max_words: usize) -> bool {
    let name = name.trim();
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.is_empty() || words.len() > max_words {
        return false;
    }
    if !name.chars().next().is_some_and(char::is_uppercase) {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    !NOT_A_NAME.contains(&lower.as_str())
}

/// The first valid span that follows any occurrence of any marker, markers tried in order.
fn capture(
    text: &str,
    markers: &[&str],
    stops: &[&str],
    max_words: usize,
    keep_comma: bool,
) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    for m in markers {
        let mut from = 0;
        while let Some(i) = lower[from..].find(m) {
            let start = from + i + m.len();
            let mut rest = &text[start..];
            for article in ["the ", "The ", "a ", "an "] {
                if let Some(r) = rest.strip_prefix(article) {
                    rest = r;
                    break;
                }
            }
            let span = trim_span(rest, stops, keep_comma);
            if valid(&span, max_words) {
                return Some(span);
            }
            from = start;
        }
    }
    None
}

/// Strip the words a sender name adds to a company name: "Acme Careers", "Orbit Talent Team".
fn strip_sender_words(name: &str) -> Option<String> {
    const TAIL: &[&str] = &[
        "recruiting",
        "recruitment",
        "recruiter",
        "recruiters",
        "careers",
        "talent",
        "acquisition",
        "hiring",
        "hr",
        "jobs",
        "team",
        "notifications",
        "people",
        "staffing",
        "via",
        "greenhouse",
        "lever",
        "workday",
        "no-reply",
        "noreply",
        "support",
        "the",
    ];
    let name = name.split(" via ").next().unwrap_or(name);
    let name = name.split(" (").next().unwrap_or(name);
    let mut words: Vec<&str> = name.split_whitespace().collect();
    let before = words.len();
    while words
        .last()
        .is_some_and(|w| TAIL.contains(&w.to_ascii_lowercase().as_str()))
    {
        words.pop();
    }
    let out = words.join(" ");
    (valid(&out, 5))
        .then_some(out)
        .filter(|_| words.len() <= before)
}

fn title_case(label: &str) -> String {
    label
        .split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `acme-corp.com` becomes `Acme Corp`; mailbox providers and ATS hosts give nothing.
pub fn company_from_domain(domain: &str) -> Option<String> {
    let in_list = |list: &[&str]| {
        list.iter()
            .any(|d| domain == *d || domain.ends_with(&format!(".{d}")))
    };
    if in_list(GENERIC_DOMAINS) || in_list(KNOWN_ATS_DOMAINS) {
        return None;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return None;
    }
    let two_part = matches!(
        labels[labels.len() - 2..].join(".").as_str(),
        "co.uk" | "com.au" | "co.nz" | "co.in" | "com.br" | "co.jp"
    );
    let i = labels.len() - if two_part { 3 } else { 2 };
    let label = labels.get(i)?;
    let out = title_case(label);
    (out.len() >= 3).then_some(out)
}

/// A signature such as "Dana Reed, Senior Recruiter, Acme Analytics" or "Orbit Talent Team".
fn company_from_signature(body: &str) -> Option<String> {
    const TITLE_WORDS: &[&str] = &[
        "recruiter",
        "sourcer",
        "talent",
        "manager",
        "director",
        "partner",
        "coordinator",
        "lead",
        "specialist",
        "hr",
        "people",
    ];
    let lines: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let tail = &lines[lines.len().saturating_sub(6)..];
    for line in tail.iter().rev() {
        if line.len() > 90 {
            continue;
        }
        let parts: Vec<&str> = line
            .split(['|', ','])
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        for w in parts.windows(2) {
            let l = w[0].to_ascii_lowercase();
            if TITLE_WORDS.iter().any(|t| l.contains(t)) && valid(w[1], 5) {
                let lower = w[1].to_ascii_lowercase();
                if !TITLE_WORDS.iter().any(|t| lower.contains(t)) {
                    return Some(w[1].to_string());
                }
            }
        }
        let lower = line.to_ascii_lowercase();
        if [
            "talent team",
            "recruiting team",
            "recruiting",
            "careers",
            "people team",
            "hiring team",
            "talent acquisition",
        ]
        .iter()
        .any(|s| lower.ends_with(s))
        {
            if let Some(name) = strip_sender_words(line) {
                return Some(name);
            }
        }
    }
    None
}

const COMPANY_MARKERS: &[&str] = &[
    "on behalf of ",
    "recruiter at ",
    "recruiting at ",
    "recruiting for ",
    "talent at ",
    "team at ",
    "role at ",
    "position at ",
    "opportunity at ",
    "opening at ",
    "hiring at ",
    "careers at ",
    "join us at ",
    "work at ",
    "work for ",
    "interview at ",
    "interview with ",
    "your interest in ",
    "joining ",
    " at ",
];

const TITLE_MARKERS: &[&str] = &[
    "job title: ",
    "position title: ",
    "position: ",
    "role: ",
    "application for the ",
    "application for ",
    "applied for the ",
    "applied for ",
    "applying for the ",
    "applying for ",
    "interview for the ",
    "interview for ",
    "interviewing for the ",
    "interviewing for ",
    "invitation: ",
    "invite: ",
    "employment, ",
    "offer: ",
    "opportunity for a ",
    "opportunity for the ",
    "position of ",
    "hiring a ",
    "hiring an ",
    "looking for a ",
    "looking for an ",
    "applying to the ",
    "applied to the ",
];

const PLACE_MARKERS: &[&str] = &["location: ", "based in ", "located in ", "office in "];

pub fn extract(mail: &ParsedMail) -> Extracted {
    let body = crate::label::clip(&mail.body, 4000);
    let all = format!("{}\n{}", mail.subject, body);

    let company = capture(&mail.subject, COMPANY_MARKERS, COMPANY_STOPS, 5, false)
        .or_else(|| capture(body, COMPANY_MARKERS, COMPANY_STOPS, 5, false))
        .or_else(|| company_from_signature(body))
        .or_else(|| {
            let name = mail.from_name.as_deref()?;
            let ats = [mail.from_domain(), mail.reply_to_domain()]
                .into_iter()
                .flatten()
                .any(|d| {
                    KNOWN_ATS_DOMAINS
                        .iter()
                        .any(|a| d == *a || d.ends_with(&format!(".{a}")))
                });
            let lower = name.to_ascii_lowercase();
            let hiring_name = ["careers", "recruit", "talent", "hiring", " jobs", " hr"]
                .iter()
                .any(|w| lower.contains(w));
            (ats || hiring_name)
                .then(|| strip_sender_words(name))
                .flatten()
        })
        .or_else(|| company_from_domain(&mail.from_domain()?));
    let title = capture(&mail.subject, TITLE_MARKERS, TITLE_STOPS, 7, false)
        .or_else(|| capture(body, TITLE_MARKERS, TITLE_STOPS, 7, false));
    let location = capture(&all, PLACE_MARKERS, &[" for ", " and "], 4, true);
    Extracted {
        company,
        title,
        location,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mail(from_name: Option<&str>, from: &str, subject: &str, body: &str) -> ParsedMail {
        ParsedMail {
            message_id: None,
            references: vec![],
            from_name: from_name.map(String::from),
            from_address: Some(from.into()),
            reply_to: None,
            subject: subject.into(),
            sent_at: None,
            body: body.into(),
        }
    }

    #[test]
    fn subject_with_title_and_company() {
        let e = extract(&mail(
            None,
            "x@gmail.com",
            "Interview invitation: Product Manager at Acme Robotics",
            "",
        ));
        assert_eq!(e.company.as_deref(), Some("Acme Robotics"));
        assert_eq!(e.title.as_deref(), Some("Product Manager"));
    }

    #[test]
    fn body_names_the_company_after_the_position() {
        let e = extract(&mail(
            None,
            "noreply@myworkday.com",
            "We received your application",
            "Hi Nadia,\n\nThank you for applying to the Backend Developer position at Orbit Labs. We have received it.",
        ));
        assert_eq!(e.company.as_deref(), Some("Orbit Labs"));
    }

    #[test]
    fn ats_sender_name_is_the_company() {
        let e = extract(&mail(
            Some("Orbit Software Talent Team"),
            "no-reply@greenhouse.io",
            "Your application to Orbit Software",
            "Thank you for interviewing with us. We have decided to pursue other candidates.",
        ));
        assert_eq!(e.company.as_deref(), Some("Orbit Software"));
    }

    #[test]
    fn signature_line_gives_the_company() {
        let e = extract(&mail(
            Some("Dana Reed"),
            "dana@gmail.com",
            "Quick chat?",
            "Hi Maria,\n\nWe have a data analyst role I think you'd like.\n\nBest,\nDana Reed, Senior Recruiter, Brightpath Health\n",
        ));
        assert_eq!(e.company.as_deref(), Some("Brightpath Health"));
    }

    #[test]
    fn company_domain_is_the_last_resort_and_mailbox_domains_give_nothing() {
        let e = extract(&mail(
            None,
            "dana@acme-corp.com",
            "Hello",
            "Are you open to a role?",
        ));
        assert_eq!(e.company.as_deref(), Some("Acme Corp"));
        let e = extract(&mail(
            None,
            "dana@gmail.com",
            "Hello",
            "Are you open to a role?",
        ));
        assert_eq!(e.company, None);
        let e = extract(&mail(None, "dana@hr.example.co.uk", "Hello", ""));
        assert_eq!(e.company.as_deref(), Some("Example"));
    }

    #[test]
    fn times_are_not_companies() {
        let e = extract(&mail(None, "x@gmail.com", "Lunch", "Meet me at 2pm."));
        assert_eq!(e.company, None);
    }

    #[test]
    fn title_from_application_for_and_job_title_label() {
        let e = extract(&mail(
            None,
            "x@y.com",
            "Re: your application for Senior Data Engineer",
            "",
        ));
        assert_eq!(e.title.as_deref(), Some("Senior Data Engineer"));
        let e = extract(&mail(
            None,
            "x@y.com",
            "Update",
            "Job title: Staff Designer\nLocation: Austin, TX\n",
        ));
        assert_eq!(e.title.as_deref(), Some("Staff Designer"));
        assert_eq!(e.location.as_deref(), Some("Austin, TX"));
    }

    #[test]
    fn nothing_found_gives_none() {
        let e = extract(&mail(None, "x@gmail.com", "Hi", "ok thanks"));
        assert_eq!(e, Extracted::default());
    }
}
