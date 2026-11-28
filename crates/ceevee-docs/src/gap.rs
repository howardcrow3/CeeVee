//! The keyword gap for one job: which skills and tools the posting asks for, and for each one
//! whether the resume already shows it, whether only the person's profile has it, or whether
//! neither does. Nothing is invented: a term the profile lacks is reported as a question for the
//! person, never added to a resume.

use serde::Serialize;

/// Where a wanted term shows up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Status {
    /// Already on the resume.
    Shown,
    /// In the person's profile but not on this resume, so it can be added honestly.
    InProfile,
    /// In neither. Only worth adding if it is true and the profile is missing it.
    Missing,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Shown => "shown",
            Status::InProfile => "profile",
            Status::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Term {
    pub term: String,
    pub status: Status,
}

/// Common skills, tools and methods that postings ask for. Matched as whole words, so "go" is not
/// in the list on purpose. The person's own skills are added to this when checking a posting.
const VOCAB: &[&str] = &[
    "sql",
    "python",
    "r",
    "excel",
    "tableau",
    "power bi",
    "looker",
    "dbt",
    "snowflake",
    "bigquery",
    "redshift",
    "databricks",
    "spark",
    "airflow",
    "pandas",
    "numpy",
    "scikit-learn",
    "tensorflow",
    "pytorch",
    "machine learning",
    "deep learning",
    "statistics",
    "a/b testing",
    "experimentation",
    "forecasting",
    "data modeling",
    "etl",
    "data visualization",
    "java",
    "javascript",
    "typescript",
    "react",
    "node.js",
    "vue",
    "angular",
    "html",
    "css",
    "c++",
    "c#",
    ".net",
    "rust",
    "golang",
    "kotlin",
    "swift",
    "ruby",
    "rails",
    "php",
    "django",
    "flask",
    "spring",
    "graphql",
    "rest api",
    "microservices",
    "kubernetes",
    "docker",
    "terraform",
    "aws",
    "azure",
    "gcp",
    "linux",
    "ci/cd",
    "git",
    "jenkins",
    "postgresql",
    "mysql",
    "mongodb",
    "redis",
    "kafka",
    "elasticsearch",
    "agile",
    "scrum",
    "kanban",
    "jira",
    "confluence",
    "roadmap",
    "roadmapping",
    "okrs",
    "kpis",
    "user research",
    "product discovery",
    "stakeholder management",
    "go-to-market",
    "pricing",
    "prototyping",
    "figma",
    "sketch",
    "wireframing",
    "design systems",
    "accessibility",
    "ux",
    "salesforce",
    "hubspot",
    "crm",
    "account management",
    "lead generation",
    "cold calling",
    "negotiation",
    "forecasting",
    "quota",
    "saas",
    "b2b",
    "b2c",
    "seo",
    "sem",
    "google analytics",
    "content marketing",
    "email marketing",
    "marketing automation",
    "copywriting",
    "paid media",
    "social media",
    "brand strategy",
    "financial modeling",
    "budgeting",
    "gaap",
    "audit",
    "accounts payable",
    "accounts receivable",
    "reconciliation",
    "month-end close",
    "netsuite",
    "quickbooks",
    "sap",
    "erp",
    "payroll",
    "recruiting",
    "onboarding",
    "employee relations",
    "hris",
    "workday",
    "compliance",
    "risk management",
    "project management",
    "pmp",
    "six sigma",
    "lean",
    "supply chain",
    "logistics",
    "procurement",
    "inventory",
    "customer success",
    "customer support",
    "zendesk",
    "technical writing",
    "public speaking",
    "mentoring",
    "cross-functional",
    "leadership",
    "team management",
    "hiring",
    "coaching",
];

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// True when `term` appears in `text` with no letter or digit touching either end. Both must
/// already be lowercase.
fn mentions(text: &str, term: &str) -> bool {
    let mut from = 0;
    while let Some(at) = text[from..].find(term) {
        let start = from + at;
        let end = start + term.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        if !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char) {
            return true;
        }
        from = start + term.chars().next().map_or(1, char::len_utf8);
        if from >= text.len() {
            break;
        }
    }
    false
}

/// The terms the posting asks for and where each stands. Terms the resume already shows come
/// last; the ones the person can honestly add come first.
pub fn analyze(
    posting: &str,
    resume_text: &str,
    profile_text: &str,
    profile_skills: &[String],
) -> Vec<Term> {
    let posting = posting.to_lowercase();
    let resume = resume_text.to_lowercase();
    let profile = profile_text.to_lowercase();
    let mut vocab: Vec<String> = VOCAB.iter().map(|s| (*s).to_string()).collect();
    for s in profile_skills {
        let s = s.trim().to_lowercase();
        if !s.is_empty() && !vocab.contains(&s) {
            vocab.push(s);
        }
    }
    let mut out: Vec<Term> = vocab
        .into_iter()
        .filter(|t| mentions(&posting, t))
        .map(|t| {
            let status = if mentions(&resume, &t) {
                Status::Shown
            } else if mentions(&profile, &t) {
                Status::InProfile
            } else {
                Status::Missing
            };
            Term { term: t, status }
        })
        .collect();
    let rank = |s: Status| match s {
        Status::InProfile => 0,
        Status::Missing => 1,
        Status::Shown => 2,
    };
    out.sort_by(|a, b| {
        rank(a.status)
            .cmp(&rank(b.status))
            .then(a.term.cmp(&b.term))
    });
    out.dedup_by(|a, b| a.term == b.term);
    out
}

#[cfg(test)]
mod tests;
