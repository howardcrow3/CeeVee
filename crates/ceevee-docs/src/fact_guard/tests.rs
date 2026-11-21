use serde_json::json;

use super::*;

const PROFILE: &str = "Maria Lopez. Senior Analyst at Northfield Logistics since March 2021. \
Built Tableau dashboards and cut report time from two days to four hours. SQL, Excel, Python. \
B.S. Industrial Engineering, Ohio State University, 2018. Managed reports for 40 depots.";
const POSTING: &str = "Acme Analytics is hiring a Data Analyst. Write SQL and build Tableau dashboards. 2+ years required.";

fn names(flags: &[Flag]) -> Vec<&str> {
    flags.iter().map(|f| f.text.as_str()).collect()
}

#[test]
fn a_letter_that_sticks_to_the_sources_has_no_flags() {
    let letter = "Dear Acme Analytics Hiring Team,\n\nI am applying for the Data Analyst role. At Northfield Logistics I built \
Tableau dashboards and managed reports for 40 depots. I hold a B.S. from Ohio State University (2018).\n\nSincerely,\nMaria Lopez";
    let flags = check(letter, &[PROFILE, POSTING]);
    assert!(flags.is_empty(), "{flags:?}");
}

#[test]
fn invented_numbers_and_employers_are_flagged_with_context() {
    let letter = "I led a team of 12 at Globex Corporation and grew revenue 35% using AWS.";
    let flags = check(letter, &[PROFILE, POSTING]);
    assert_eq!(
        names(&flags),
        vec!["12", "Globex", "Corporation", "35", "AWS"]
    );
    let g = flags.iter().find(|f| f.text == "Globex").unwrap();
    assert_eq!(g.kind, FlagKind::Name);
    assert!(
        g.context.contains("team of 12 at Globex Corporation")
            || g.context.contains("at Globex Corporation"),
        "{}",
        g.context
    );
}

#[test]
fn numbers_match_across_formatting() {
    assert!(check(
        "Saved $1,000 on 40 depots",
        &["We saved 1000 across 40 depots"]
    )
    .is_empty());
    let flags = check("Saved $2,000", &["We saved 1000"]);
    assert_eq!(names(&flags), vec!["2,000"]);
}

#[test]
fn sentence_starts_and_common_letter_words_are_not_names() {
    let flags = check(
        "Thanks for reading. Experienced people join quickly. Dear Sir, regards.",
        &[PROFILE],
    );
    assert!(flags.is_empty(), "{flags:?}");
}

#[test]
fn possessives_and_each_flag_appears_once() {
    assert!(check("I enjoyed Northfield's culture.", &[PROFILE]).is_empty());
    let flags = check("I worked with Globex and, again, with Globex.", &[PROFILE]);
    assert_eq!(names(&flags), vec!["Globex",]);
}

#[test]
fn resume_json_is_checked_through_its_text() {
    let resume = json!({
        "work": [{"company": "Northfield Logistics", "title": "Senior Analyst", "start": "2021-03", "highlights": ["Cut report time to four hours"]}],
        "skills": ["SQL", "Kubernetes"]
    });
    let flags = check_value(&resume, &[PROFILE, POSTING]);
    assert_eq!(names(&flags), vec!["Kubernetes"]);
}

#[test]
fn year_months_and_ranges() {
    // 2021-03 is fine when 2021 is known; the month is not compared.
    assert!(check("Since 2021-03", &["since March 2021"]).is_empty());
    assert!(check("2018-2021", &["from 2018 to 2021"]).is_empty());
    let flags = check("2018-2022", &["from 2018 to 2021"]);
    assert_eq!(names(&flags), vec!["2018-2022"]);
}
