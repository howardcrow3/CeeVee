//! The no-model candidate filter: decide which mail is worth fetching and labelling.

use crate::parse::ParsedMail;
use std::collections::HashSet;

/// Sender domains of the big applicant tracking systems and job-alert services.
pub const KNOWN_ATS_DOMAINS: &[&str] = &[
    "greenhouse.io",
    "lever.co",
    "ashbyhq.com",
    "myworkday.com",
    "myworkdayjobs.com",
    "icims.com",
    "smartrecruiters.com",
    "jobvite.com",
    "workable.com",
    "taleo.net",
    "successfactors.com",
    "bamboohr.com",
    "recruitee.com",
    "breezy.hr",
];

pub const KNOWN_ALERT_DOMAINS: &[&str] = &[
    "linkedin.com",
    "indeed.com",
    "ziprecruiter.com",
    "glassdoor.com",
    "wellfound.com",
    "builtin.com",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Candidate {
    /// From a company we track or an ATS, or a reply in a thread we started.
    Employer,
    /// A job-alert email, which goes to the alert parsers.
    Alert,
}

#[derive(Debug, Default)]
pub struct Rules {
    /// Domains of companies in the database.
    pub company_domains: HashSet<String>,
    /// Message IDs of mail the user sent from CeeVee.
    pub sent_message_ids: HashSet<String>,
}

fn matches(domain: &str, list: &[&str]) -> bool {
    list.iter()
        .any(|d| domain == *d || domain.ends_with(&format!(".{d}")))
}

fn in_set(domain: &str, set: &HashSet<String>) -> bool {
    set.iter()
        .any(|d| domain == d || domain.ends_with(&format!(".{d}")))
}

pub fn classify(mail: &ParsedMail, rules: &Rules) -> Option<Candidate> {
    let domains = [mail.from_domain(), mail.reply_to_domain()];
    let domains: Vec<&String> = domains.iter().flatten().collect();
    if mail
        .references
        .iter()
        .any(|r| rules.sent_message_ids.contains(r))
    {
        return Some(Candidate::Employer);
    }
    if domains
        .iter()
        .any(|d| in_set(d, &rules.company_domains) || matches(d, KNOWN_ATS_DOMAINS))
    {
        return Some(Candidate::Employer);
    }
    if domains.iter().any(|d| matches(d, KNOWN_ALERT_DOMAINS)) {
        return Some(Candidate::Alert);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mail(from: &str, reply_to: Option<&str>, refs: &[&str]) -> ParsedMail {
        ParsedMail {
            message_id: None,
            references: refs.iter().map(|s| s.to_string()).collect(),
            from_name: None,
            from_address: Some(from.into()),
            reply_to: reply_to.map(String::from),
            subject: String::new(),
            sent_at: None,
            body: String::new(),
        }
    }

    fn rules() -> Rules {
        Rules {
            company_domains: ["acme.com".to_string()].into(),
            sent_message_ids: ["mine@x".to_string()].into(),
        }
    }

    #[test]
    fn company_domain_and_subdomain_match() {
        assert_eq!(
            classify(&mail("a@acme.com", None, &[]), &rules()),
            Some(Candidate::Employer)
        );
        assert_eq!(
            classify(&mail("a@hr.acme.com", None, &[]), &rules()),
            Some(Candidate::Employer)
        );
        assert_eq!(classify(&mail("a@notacme.com", None, &[]), &rules()), None);
    }

    #[test]
    fn ats_sender_or_reply_to_matches() {
        assert_eq!(
            classify(&mail("no-reply@us.greenhouse.io", None, &[]), &rules()),
            Some(Candidate::Employer)
        );
        assert_eq!(
            classify(&mail("x@random.com", Some("a@lever.co"), &[]), &rules()),
            Some(Candidate::Employer)
        );
    }

    #[test]
    fn reply_to_our_sent_mail_matches() {
        assert_eq!(
            classify(&mail("p@gmail.com", None, &["mine@x"]), &rules()),
            Some(Candidate::Employer)
        );
    }

    #[test]
    fn alerts_are_routed_separately() {
        assert_eq!(
            classify(&mail("jobs-noreply@linkedin.com", None, &[]), &rules()),
            Some(Candidate::Alert)
        );
    }

    #[test]
    fn everything_else_is_ignored() {
        assert_eq!(
            classify(&mail("friend@gmail.com", None, &[]), &rules()),
            None
        );
    }
}
