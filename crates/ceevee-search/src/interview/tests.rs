use super::*;

#[test]
fn a_one_hour_hold_with_a_reminder() {
    let f = calendar_file(7, "Analyst", Some("Acme"), "2026-12-03T17:00", None).unwrap();
    assert!(f.contains("DTSTART:20261203T170000\r\n"));
    assert!(f.contains("DTEND:20261203T180000\r\n"));
    assert!(f.contains("SUMMARY:Interview: Analyst at Acme\r\n"));
    assert!(f.contains("UID:ceevee-interview-7@ceevee.local"));
    assert!(f.contains("TRIGGER:-PT1H"));
}

#[test]
fn the_hour_rolls_over_midnight_and_the_new_year() {
    let f = calendar_file(1, "A", None, "2026-12-31T23:30", None).unwrap();
    assert!(f.contains("DTEND:20270101T003000\r\n"));
    let f = calendar_file(1, "A", None, "2028-02-28T23:30", None).unwrap();
    assert!(f.contains("DTEND:20280229T003000\r\n"));
}

#[test]
fn text_is_escaped_and_the_posting_link_is_kept() {
    let f = calendar_file(
        1,
        "Sales; East",
        Some("A, B"),
        "2026-12-03T09:00",
        Some("https://x.test/j"),
    )
    .unwrap();
    assert!(f.contains("SUMMARY:Interview: Sales\\; East at A\\, B"));
    assert!(f.contains("Posting: https://x.test/j"));
}

#[test]
fn a_time_that_is_not_a_time_gives_nothing() {
    assert_eq!(calendar_file(1, "A", None, "next week", None), None);
    assert_eq!(calendar_file(1, "A", None, "2026-13-03T09:00", None), None);
}
