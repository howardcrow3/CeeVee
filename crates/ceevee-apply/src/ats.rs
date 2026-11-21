//! Recognise the applicant tracking system behind an application URL.

use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ats {
    Greenhouse,
    Lever,
    Ashby,
    Workday,
    SmartRecruiters,
    Icims,
    Workable,
    /// Not recognised: the generic form filler handles it.
    Unknown,
}

impl Ats {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Greenhouse => "greenhouse",
            Self::Lever => "lever",
            Self::Ashby => "ashby",
            Self::Workday => "workday",
            Self::SmartRecruiters => "smartrecruiters",
            Self::Icims => "icims",
            Self::Workable => "workable",
            Self::Unknown => "unknown",
        }
    }
}

fn host_is(host: &str, domain: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

pub fn detect(url: &str) -> Ats {
    let Ok(u) = Url::parse(url) else {
        return Ats::Unknown;
    };
    let Some(host) = u.host_str().map(str::to_ascii_lowercase) else {
        return Ats::Unknown;
    };
    const TABLE: &[(&str, Ats)] = &[
        ("greenhouse.io", Ats::Greenhouse),
        ("lever.co", Ats::Lever),
        ("ashbyhq.com", Ats::Ashby),
        ("myworkdayjobs.com", Ats::Workday),
        ("myworkdaysite.com", Ats::Workday),
        ("smartrecruiters.com", Ats::SmartRecruiters),
        ("icims.com", Ats::Icims),
        ("workable.com", Ats::Workable),
    ];
    TABLE
        .iter()
        .find(|(d, _)| host_is(&host, d))
        .map_or(Ats::Unknown, |(_, a)| *a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_systems() {
        assert_eq!(
            detect("https://boards.greenhouse.io/acme/jobs/123"),
            Ats::Greenhouse
        );
        assert_eq!(
            detect("https://job-boards.greenhouse.io/acme/jobs/1"),
            Ats::Greenhouse
        );
        assert_eq!(detect("https://jobs.lever.co/acme/abc/apply"), Ats::Lever);
        assert_eq!(detect("https://jobs.ashbyhq.com/acme/uuid"), Ats::Ashby);
        assert_eq!(
            detect("https://acme.wd5.myworkdayjobs.com/en-US/careers/job/x"),
            Ats::Workday
        );
        assert_eq!(detect("https://acme.icims.com/jobs/1/job"), Ats::Icims);
    }

    #[test]
    fn lookalike_hosts_are_not_matched() {
        assert_eq!(detect("https://notgreenhouse.io/x"), Ats::Unknown);
        assert_eq!(detect("https://greenhouse.io.evil.com/x"), Ats::Unknown);
        assert_eq!(detect("not a url"), Ats::Unknown);
    }
}
