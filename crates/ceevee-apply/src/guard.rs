//! Guardrails for automation: pages it must never visit and controls that mean "submit".

use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    LinkedInMessaging,
    LinkedInEasyApply,
    IndeedApply,
    ZipRecruiterOneClick,
    AccountCreation,
    Captcha,
}

impl Refusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::LinkedInMessaging => "CeeVee never automates LinkedIn messaging",
            Self::LinkedInEasyApply => "CeeVee never automates LinkedIn Easy Apply",
            Self::IndeedApply => "CeeVee never automates Indeed Apply",
            Self::ZipRecruiterOneClick => "CeeVee never automates ZipRecruiter 1-Click",
            Self::AccountCreation => "creating accounts is left to you",
            Self::Captcha => "CAPTCHA pages are left to you",
        }
    }
}

fn host_is(host: &str, domain: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// `Err` when automation must not go to this page. `page_text` is optional visible text, used to
/// spot CAPTCHA and sign-up pages.
pub fn check_navigation(url: &str, page_text: Option<&str>) -> Result<(), Refusal> {
    if let Ok(u) = Url::parse(url) {
        let host = u.host_str().unwrap_or("").to_ascii_lowercase();
        let path = u.path().to_ascii_lowercase();
        if host_is(&host, "linkedin.com") {
            if path.starts_with("/messaging") {
                return Err(Refusal::LinkedInMessaging);
            }
            if path.contains("/apply") || u.query().is_some_and(|q| q.contains("openSDUIApplyFlow"))
            {
                return Err(Refusal::LinkedInEasyApply);
            }
        }
        if host_is(&host, "indeed.com") && (path.contains("/apply") || host.starts_with("apply.")) {
            return Err(Refusal::IndeedApply);
        }
        if host_is(&host, "ziprecruiter.com") && path.contains("1-click") {
            return Err(Refusal::ZipRecruiterOneClick);
        }
        if path.contains("/signup") || path.contains("/register") || path.contains("create-account")
        {
            return Err(Refusal::AccountCreation);
        }
    }
    if let Some(t) = page_text {
        let t = t.to_ascii_lowercase();
        if t.contains("captcha")
            || t.contains("verify you are human")
            || t.contains("i'm not a robot")
        {
            return Err(Refusal::Captcha);
        }
    }
    Ok(())
}

/// True when clicking a control with this text submits an application.
pub fn is_submit_control(text: &str) -> bool {
    let t = text.trim().to_ascii_lowercase();
    [
        "submit application",
        "submit",
        "apply now",
        "send application",
        "finish application",
    ]
    .iter()
    .any(|p| t == *p || t.starts_with(&format!("{p} ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_list_pages_are_refused() {
        assert_eq!(
            check_navigation("https://www.linkedin.com/messaging/thread/1", None),
            Err(Refusal::LinkedInMessaging)
        );
        assert_eq!(
            check_navigation("https://www.linkedin.com/jobs/view/1/apply", None),
            Err(Refusal::LinkedInEasyApply)
        );
        assert_eq!(
            check_navigation("https://apply.indeed.com/x", None),
            Err(Refusal::IndeedApply)
        );
        assert_eq!(
            check_navigation("https://www.ziprecruiter.com/apply/1-click/9", None),
            Err(Refusal::ZipRecruiterOneClick)
        );
        assert_eq!(
            check_navigation("https://jobs.acme.com/signup", None),
            Err(Refusal::AccountCreation)
        );
        assert_eq!(
            check_navigation(
                "https://jobs.acme.com/apply",
                Some("Please verify you are human")
            ),
            Err(Refusal::Captcha)
        );
    }

    #[test]
    fn ordinary_pages_pass() {
        assert_eq!(
            check_navigation(
                "https://boards.greenhouse.io/acme/jobs/1",
                Some("Apply for this job")
            ),
            Ok(())
        );
        assert_eq!(
            check_navigation("https://www.linkedin.com/jobs/view/1", None),
            Ok(())
        );
    }

    #[test]
    fn submit_detection() {
        assert!(is_submit_control("Submit Application"));
        assert!(is_submit_control(" Submit "));
        assert!(is_submit_control("Apply now"));
        assert!(!is_submit_control("Next"));
        assert!(!is_submit_control("Save draft"));
        assert!(!is_submit_control("Attach resume"));
    }
}
