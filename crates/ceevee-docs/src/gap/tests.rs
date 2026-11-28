use super::*;

fn status_of(terms: &[Term], t: &str) -> Option<Status> {
    terms.iter().find(|x| x.term == t).map(|x| x.status)
}

#[test]
fn sorts_wanted_terms_by_what_the_person_can_do() {
    let posting = "We use SQL, Python and Tableau daily. Kubernetes is a plus.";
    let resume = "Built dashboards in Tableau. Wrote Python scripts.";
    let profile = "Analyst. Skills: SQL, Python, Tableau.";
    let t = analyze(posting, resume, profile, &[]);
    assert_eq!(status_of(&t, "sql"), Some(Status::InProfile));
    assert_eq!(status_of(&t, "python"), Some(Status::Shown));
    assert_eq!(status_of(&t, "tableau"), Some(Status::Shown));
    assert_eq!(status_of(&t, "kubernetes"), Some(Status::Missing));
    // Honest additions first, then questions, then what is already shown.
    assert_eq!(t[0].term, "sql");
    assert_eq!(t[1].term, "kubernetes");
    assert_eq!(t.last().unwrap().status, Status::Shown);
}

#[test]
fn whole_words_only() {
    let t = analyze(
        "Experience with Rust required. Trusting teams.",
        "",
        "",
        &[],
    );
    assert_eq!(status_of(&t, "rust"), Some(Status::Missing));
    let t = analyze("Trusting teams, frustrated users", "", "", &[]);
    assert!(t.is_empty());
    // "r" must not match inside words.
    let t = analyze("Our team loves reports", "", "", &[]);
    assert!(t.is_empty());
}

#[test]
fn terms_with_symbols_match() {
    let t = analyze("C++ and C# and Node.js, plus CI/CD.", "I know c++", "", &[]);
    assert_eq!(status_of(&t, "c++"), Some(Status::Shown));
    assert_eq!(status_of(&t, "c#"), Some(Status::Missing));
    assert_eq!(status_of(&t, "node.js"), Some(Status::Missing));
    assert_eq!(status_of(&t, "ci/cd"), Some(Status::Missing));
}

#[test]
fn the_persons_own_skills_count_as_wanted_terms() {
    let t = analyze(
        "Familiarity with Looker Studio and Bloomberg Terminal.",
        "",
        "Skills: Bloomberg Terminal",
        &["Bloomberg Terminal".to_string()],
    );
    assert_eq!(status_of(&t, "bloomberg terminal"), Some(Status::InProfile));
    assert_eq!(status_of(&t, "looker"), Some(Status::Missing));
}
