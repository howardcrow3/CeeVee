use serde_json::json;

use super::*;

fn compile(v: Value) -> String {
    json_schema_to_gbnf(&v).unwrap()
}

#[test]
fn flat_object_with_required_and_optional_properties() {
    let g = compile(json!({
        "type": "object",
        "properties": {"name": {"type": "string"}, "age": {"type": "integer"}, "tag": {"type": "string"}},
        "required": ["name", "age"]
    }));
    assert!(g.contains(r#"root ::= "{" ws "\"age\"" ws ":" ws integer "," ws "\"name\"" ws ":" ws string ("," ws "\"tag\"" ws ":" ws string)? "}" ws"#), "{g}");
    assert!(g.contains("ws ::="));
}

#[test]
fn enums_consts_and_nullable_types() {
    let g = compile(json!({"type": "object", "properties": {
        "level": {"enum": ["junior", "senior"]},
        "kind": {"const": "x"},
        "pay": {"type": ["integer", "null"]}
    }, "required": ["level", "kind", "pay"]}));
    assert!(g.contains(r#"("\"junior\"" | "\"senior\"") ws"#), "{g}");
    assert!(g.contains(r#""\"x\"" ws"#), "{g}");
    assert!(g.contains("(integer | null)"), "{g}");
}

#[test]
fn arrays_with_and_without_minimum() {
    let g = compile(json!({"type": "array", "items": {"type": "string"}}));
    assert!(
        g.contains(r#"root ::= "[" ws (root-item-1 ("," ws root-item-1)*)? "]" ws"#),
        "{g}"
    );
    let g = compile(json!({"type": "array", "items": {"type": "string"}, "minItems": 1}));
    assert!(
        g.contains(r#""[" ws root-item-1 ("," ws root-item-1)* "]" ws"#),
        "{g}"
    );
}

#[test]
fn refs_and_recursion_terminate() {
    let g = compile(json!({
        "$ref": "#/$defs/Node",
        "$defs": {"Node": {"type": "object", "properties": {
            "value": {"type": "integer"},
            "children": {"type": "array", "items": {"$ref": "#/$defs/Node"}}
        }, "required": ["value", "children"]}}
    }));
    assert!(g.contains("def-Node ::="), "{g}");
    assert!(g.matches("def-Node").count() >= 3);
}

#[test]
fn objects_with_only_optional_properties_can_start_anywhere_or_be_empty() {
    let g = compile(
        json!({"type": "object", "properties": {"a": {"type": "string"}, "b": {"type": "string"}}}),
    );
    let root = g.lines().find(|l| l.starts_with("root ::=")).unwrap();
    assert!(root.contains(" | "), "{root}");
    assert!(
        root.contains(r#"("\"b\"""#) || root.contains(r#""\"b\"""#),
        "{root}"
    );
}

#[test]
fn any_of_becomes_alternatives() {
    let g = compile(json!({"anyOf": [{"type": "string"}, {"type": "null"}]}));
    assert!(g.contains("root ::= (string | null)"), "{g}");
}

#[test]
fn rejects_what_it_cannot_enforce() {
    assert!(json_schema_to_gbnf(&json!({})).is_err());
    assert!(json_schema_to_gbnf(&json!({"type": "array"})).is_err());
    assert!(json_schema_to_gbnf(&json!({"$ref": "https://example.com/x.json"})).is_err());
    assert!(json_schema_to_gbnf(&json!({"$ref": "#/$defs/Missing"})).is_err());
    assert!(
        json_schema_to_gbnf(&json!({"allOf": [{"type": "string"}, {"type": "null"}]})).is_err()
    );
}

#[test]
fn literals_are_escaped() {
    assert_eq!(lit(r#"a"b\c"#), r#""a\"b\\c""#);
    let g = compile(json!({"enum": ["say \"hi\""]}));
    assert!(g.contains(r#""\"say \\\"hi\\\"\"""#), "{g}");
}
