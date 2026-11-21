use serde_json::json;

use super::*;

fn spec(name: &'static str, risk: Risk) -> ToolSpec {
    ToolSpec {
        name,
        description: "test tool",
        risk,
        input_schema: json!({
            "type": "object",
            "required": ["id"],
            "properties": {"id": {"type": "integer", "minimum": 1}}
        }),
        action: (risk == Risk::Act).then_some("apply.submit"),
        untrusted_output: false,
    }
}

fn echo(name: &'static str, risk: Risk) -> FnTool<impl Fn(Value) -> ToolFuture + Send + Sync> {
    FnTool::new(spec(name, risk), |v| {
        Box::pin(async move { Ok(json!({"got": v})) })
    })
}

#[test]
fn risk_levels_are_ordered() {
    assert!(Risk::Read < Risk::Draft && Risk::Draft < Risk::Propose && Risk::Propose < Risk::Act);
}

#[test]
fn allow_list_entries() {
    assert!(allowed("search.*", "search.run"));
    assert!(!allowed("search.*", "searchy.run"));
    assert!(!allowed("search.*", "mail.search"));
    assert!(allowed("mail.read", "mail.read"));
    assert!(!allowed("mail.read", "mail.read_thread"));
}

#[test]
fn registration_rules() {
    let mut r = Registry::new();
    r.register(echo("search.get_posting", Risk::Read)).unwrap();
    assert!(r.register(echo("search.get_posting", Risk::Read)).is_err());
    assert!(r.register(echo("noGroup", Risk::Read)).is_err());
    let no_action = FnTool::new(
        ToolSpec {
            action: None,
            ..spec("apply.submit", Risk::Act)
        },
        |v| Box::pin(async move { Ok(v) }),
    );
    assert!(r.register(no_action).is_err());
}

#[test]
fn agents_see_only_their_allow_list() {
    let mut r = Registry::new();
    for (n, k) in [
        ("search.run", Risk::Read),
        ("mail.send", Risk::Act),
        ("mail.read", Risk::Read),
    ] {
        r.register(echo(n, k)).unwrap();
    }
    let names: Vec<_> = r
        .specs_for(&["search.*", "mail.read"])
        .iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, vec!["mail.read", "search.run"]);
}

#[tokio::test]
async fn call_validates_input_first() {
    let mut r = Registry::new();
    r.register(echo("search.get_posting", Risk::Read)).unwrap();
    let ok = r
        .call("search.get_posting", json!({"id": 3}))
        .await
        .unwrap();
    assert_eq!(ok["got"]["id"], 3);
    let bad = r
        .call("search.get_posting", json!({"id": 0}))
        .await
        .unwrap_err();
    assert!(bad.to_string().contains("rejected"), "{bad}");
    let missing = r.call("nope.nothing", json!({})).await.unwrap_err();
    assert!(matches!(missing, Error::NotFound(_)));
}
