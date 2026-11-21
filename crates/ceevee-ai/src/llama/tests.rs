use super::*;

#[test]
fn utf8_split_across_tokens_is_held_back() {
    let euro = "€".as_bytes(); // three bytes
    let mut pending = euro[..2].to_vec();
    assert_eq!(take_valid_utf8(&mut pending), "");
    assert_eq!(pending.len(), 2);
    pending.push(euro[2]);
    assert_eq!(take_valid_utf8(&mut pending), "€");
    assert!(pending.is_empty());
}

#[test]
fn plain_text_passes_through_and_bad_bytes_are_replaced() {
    let mut pending = b"ab".to_vec();
    assert_eq!(take_valid_utf8(&mut pending), "ab");
    let mut bad = vec![b'a', 0xFF, b'b'];
    assert_eq!(take_valid_utf8(&mut bad), "a\u{FFFD}b");
}

/// Runs the whole path against a real GGUF file. Set CEEVEE_TEST_MODEL to a model path.
#[test]
#[ignore = "needs CEEVEE_TEST_MODEL pointing at a GGUF file"]
fn real_model_follows_a_grammar() {
    let path = std::env::var("CEEVEE_TEST_MODEL").expect("CEEVEE_TEST_MODEL");
    let mut b = LlamaCppBackend::new(path).with_chat_template(false);
    b.load().unwrap();
    assert!(b.is_loaded());
    let schema = serde_json::json!({
        "type": "object",
        "required": ["answer"],
        "properties": {"answer": {"enum": ["yes", "no"]}}
    });
    let mut req = GenerateRequest::new("Is water wet? Answer as JSON: ");
    req.grammar = Some(crate::grammar::json_schema_to_gbnf(&schema).unwrap());
    req.max_tokens = 40;
    req.temperature = 0.0;
    let mut streamed = String::new();
    let out = b
        .generate(&req, &AtomicBool::new(false), &mut |t| streamed.push_str(t))
        .unwrap();
    assert_eq!(out, streamed);
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert!(matches!(v["answer"].as_str(), Some("yes" | "no")), "{out}");
    b.unload();
    assert!(!b.is_loaded());
}
