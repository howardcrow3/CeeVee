//! Decode a raw email (MIME) into the few fields CeeVee needs.

use ceevee_core::{Error, Result};
use mail_parser::{Address, HeaderValue, MessageParser};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMail {
    pub message_id: Option<String>,
    /// Message IDs this mail replies to or references, oldest first.
    pub references: Vec<String>,
    pub from_name: Option<String>,
    pub from_address: Option<String>,
    pub reply_to: Option<String>,
    pub subject: String,
    /// Unix seconds, when the Date header parses.
    pub sent_at: Option<i64>,
    /// Plain text body; HTML-only mail is converted to text.
    pub body: String,
}

impl ParsedMail {
    /// Lower-case domain of the sender, if any.
    pub fn from_domain(&self) -> Option<String> {
        domain_of(self.from_address.as_deref()?)
    }

    pub fn reply_to_domain(&self) -> Option<String> {
        domain_of(self.reply_to.as_deref()?)
    }
}

pub fn domain_of(address: &str) -> Option<String> {
    let (_, d) = address.rsplit_once('@')?;
    let d = d.trim().trim_end_matches('>').to_ascii_lowercase();
    (!d.is_empty()).then_some(d)
}

fn ids(v: &HeaderValue<'_>) -> Vec<String> {
    match v {
        HeaderValue::Text(t) => vec![t.to_string()],
        HeaderValue::TextList(l) => l.iter().map(|t| t.to_string()).collect(),
        _ => vec![],
    }
}

fn first_address(a: Option<&Address<'_>>) -> (Option<String>, Option<String>) {
    let Some(first) = a.and_then(|a| a.first()) else {
        return (None, None);
    };
    (
        first.name().map(str::to_string),
        first.address().map(|s| s.to_ascii_lowercase()),
    )
}

pub fn parse(raw: &[u8]) -> Result<ParsedMail> {
    let msg = MessageParser::default()
        .parse(raw)
        .ok_or_else(|| Error::Invalid("not a readable email".into()))?;
    let (from_name, from_address) = first_address(msg.from());
    let (_, reply_to) = first_address(msg.reply_to());
    let mut references = ids(msg.references());
    for id in ids(msg.in_reply_to()) {
        if !references.contains(&id) {
            references.push(id);
        }
    }
    let body = msg
        .body_text(0)
        .map(|b| b.trim().to_string())
        .unwrap_or_default();
    Ok(ParsedMail {
        message_id: msg.message_id().map(str::to_string),
        references,
        from_name,
        from_address,
        reply_to,
        subject: msg.subject().unwrap_or("").trim().to_string(),
        sent_at: msg.date().map(|d| d.to_timestamp()),
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIL: &[u8] = b"From: Dana Recruiter <Dana@Acme-Corp.com>\r\n\
Reply-To: jobs@mail.greenhouse.io\r\n\
To: me@example.com\r\n\
Subject: =?UTF-8?Q?Interview_with_Acme_=E2=80=93_next_steps?=\r\n\
Message-ID: <abc@acme-corp.com>\r\n\
In-Reply-To: <orig@example.com>\r\n\
References: <first@example.com> <orig@example.com>\r\n\
Date: Mon, 10 Nov 2026 09:30:00 -0500\r\n\
Content-Type: text/plain; charset=utf-8\r\n\r\n\
Hi, we'd love to talk Thursday at 2pm.\r\n";

    #[test]
    fn decodes_headers_and_body() {
        let m = parse(MAIL).unwrap();
        assert_eq!(m.from_name.as_deref(), Some("Dana Recruiter"));
        assert_eq!(m.from_address.as_deref(), Some("dana@acme-corp.com"));
        assert_eq!(m.from_domain().as_deref(), Some("acme-corp.com"));
        assert_eq!(m.reply_to_domain().as_deref(), Some("mail.greenhouse.io"));
        assert_eq!(m.subject, "Interview with Acme \u{2013} next steps");
        assert_eq!(m.message_id.as_deref(), Some("abc@acme-corp.com"));
        assert_eq!(m.references, vec!["first@example.com", "orig@example.com"]);
        assert!(m.sent_at.is_some());
        assert!(m.body.contains("Thursday at 2pm"));
    }

    #[test]
    fn html_only_mail_becomes_text() {
        let raw = b"From: a@b.com\r\nSubject: Hi\r\nContent-Type: text/html\r\n\r\n<p>Hello <b>there</b></p>";
        let m = parse(raw).unwrap();
        assert!(m.body.contains("Hello") && m.body.contains("there"));
        assert!(!m.body.contains('<'));
    }

    #[test]
    fn domain_helper_handles_odd_input() {
        assert_eq!(domain_of("x@Y.com").as_deref(), Some("y.com"));
        assert_eq!(domain_of("nope"), None);
    }
}
