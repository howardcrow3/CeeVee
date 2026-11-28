use serde_json::json;

use super::*;

fn tool(name: &'static str, risk: Risk, action: Option<&'static str>) -> ToolSpec {
    ToolSpec {
        name,
        description: "",
        risk,
        input_schema: json!({}),
        action,
        untrusted_output: false,
    }
}

fn call(t: &ToolSpec) -> Call<'_> {
    Call {
        tool: t,
        category: None,
        user_initiated: false,
        from_untrusted: false,
    }
}

#[test]
fn read_and_draft_run_propose_queues() {
    let p = Policy::new();
    assert_eq!(
        p.decide(&call(&tool("search.get", Risk::Read, None))),
        Decision::Run
    );
    assert_eq!(
        p.decide(&call(&tool("docs.edit", Risk::Draft, None))),
        Decision::Run
    );
    assert!(matches!(
        p.decide(&call(&tool("profile.propose_change", Risk::Propose, None))),
        Decision::Queue(_)
    ));
}

#[test]
fn act_defaults_to_ask_and_follows_the_mode() {
    let t = tool("apply.submit", Risk::Act, Some("apply.submit"));
    let mut p = Policy::new();
    assert!(matches!(p.decide(&call(&t)), Decision::Queue(_)));
    p.set_global("apply.submit", Mode::Auto);
    assert_eq!(p.decide(&call(&t)), Decision::Run);
    p.set_global("apply.submit", Mode::Off);
    assert!(matches!(p.decide(&call(&t)), Decision::Deny(_)));
}

#[test]
fn category_overrides_global() {
    let t = tool("apply.submit", Risk::Act, Some("apply.submit"));
    let mut p = Policy::new();
    p.set_global("apply.submit", Mode::Auto);
    p.set_category(7, "apply.submit", Mode::Ask);
    let mut c = call(&t);
    assert_eq!(p.decide(&c), Decision::Run);
    c.category = Some(7);
    assert!(matches!(p.decide(&c), Decision::Queue(_)));
    c.category = Some(8);
    assert_eq!(p.decide(&c), Decision::Run);
}

#[test]
fn a_user_click_runs_an_act_unless_it_is_off_limits() {
    let t = tool("mail.send", Risk::Act, Some("mail.reply"));
    let mut p = Policy::new();
    p.set_global("mail.reply", Mode::Off);
    let mut c = call(&t);
    c.user_initiated = true;
    assert_eq!(p.decide(&c), Decision::Run);
    let never = tool("linkedin.send", Risk::Act, Some("linkedin.message"));
    let mut c = call(&never);
    c.user_initiated = true;
    assert!(matches!(p.decide(&c), Decision::Deny(_)));
}

#[test]
fn untrusted_input_never_reaches_auto() {
    let t = tool("apply.submit", Risk::Act, Some("apply.submit"));
    let mut p = Policy::new();
    p.set_global("apply.submit", Mode::Auto);
    let mut c = call(&t);
    c.from_untrusted = true;
    assert!(matches!(p.decide(&c), Decision::Queue(m) if m.contains("posting, email")));
}

#[test]
fn never_list_matches_whole_segments() {
    assert!(is_never("linkedin.message"));
    assert!(is_never("linkedin.message.bulk"));
    assert!(!is_never("linkedin.messages"));
    assert!(!is_never("mail.delete_draft"));
    let t = tool("data.delete", Risk::Read, None);
    assert!(matches!(Policy::new().decide(&call(&t)), Decision::Deny(_)));
}
