use super::*;

fn crit() -> Criteria {
    Criteria::default()
}

#[test]
fn three_states_from_the_score() {
    assert_eq!(Fit::from_score(100.0), Fit::Strong);
    assert_eq!(Fit::from_score(70.0), Fit::Strong);
    assert_eq!(Fit::from_score(69.0), Fit::Possible);
    assert_eq!(Fit::from_score(40.0), Fit::Possible);
    assert_eq!(Fit::from_score(39.0), Fit::Skip);
    assert_eq!(Fit::from_score(10.0).as_str(), "skip");
}

#[test]
fn reads_the_top_of_a_pay_range() {
    assert_eq!(pay_ceiling("$120k - $150k"), Some(150_000));
    assert_eq!(pay_ceiling("$120,000 to $135,500 a year"), Some(135_500));
    assert_eq!(pay_ceiling("USD 90000"), Some(90_000));
    assert_eq!(pay_ceiling("$50/hr"), Some(104_000));
    assert_eq!(pay_ceiling("Competitive"), None);
    assert_eq!(pay_ceiling("DOE, 401k match"), None);
}

#[test]
fn pay_below_the_floor_is_a_dealbreaker_but_unlisted_pay_is_not() {
    let c = Criteria {
        min_pay: 120_000,
        ..crit()
    };
    let why = dealbreaker(&c, "Analyst", None, Some("$80k - $100k")).unwrap();
    assert_eq!(why, "pays up to $100,000, below your $120,000 floor");
    assert_eq!(dealbreaker(&c, "Analyst", None, Some("$90k - $130k")), None);
    assert_eq!(dealbreaker(&c, "Analyst", None, None), None);
    assert_eq!(dealbreaker(&c, "Analyst", None, Some("Competitive")), None);
}

#[test]
fn remote_only_needs_remote_in_the_title_or_place() {
    let c = Criteria {
        remote_only: true,
        ..crit()
    };
    assert!(dealbreaker(&c, "Analyst", Some("Columbus, OH"), None).is_some());
    assert_eq!(dealbreaker(&c, "Analyst", Some("Remote (US)"), None), None);
    assert_eq!(dealbreaker(&c, "Remote Analyst", None, None), None);
    assert_eq!(
        dealbreaker(&crit(), "Analyst", Some("Columbus, OH"), None),
        None
    );
}

#[test]
fn explains_in_one_sentence() {
    let reasons = vec![
        "title matches \"product manager\"".to_string(),
        "remote".to_string(),
        "does not mention sql".to_string(),
    ];
    assert_eq!(
        explain(83.0, &reasons),
        "Strong fit: title matches \"product manager\", remote; does not mention sql."
    );
    assert_eq!(
        explain(10.0, &["not remote, and you only want remote".to_string()]),
        "Skip: not remote, and you only want remote."
    );
    assert_eq!(
        explain(
            33.0,
            &[
                "title is not one you listed".to_string(),
                "remote".to_string()
            ]
        ),
        "Skip: remote; title is not one you listed."
    );
    assert_eq!(explain(50.0, &[]), "Possible fit.");
}
