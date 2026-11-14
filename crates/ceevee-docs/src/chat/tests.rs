use super::*;

#[test]
fn prompt_names_the_format_for_each_kind() {
    let p = edit_prompt(DocKind::BaseResume, "{}", "shorter");
    assert!(p.contains("valid JSON") && p.contains("<request>\nshorter"));
    let p = edit_prompt(DocKind::CoverLetter, "Dear team", "warmer");
    assert!(p.contains("cover letter") && p.contains("plain text"));
}

#[test]
fn clean_reply_strips_fences_and_thinking() {
    assert_eq!(clean_reply("```json\n{\"a\":1}\n```"), "{\"a\":1}");
    assert_eq!(clean_reply("<think>hmm</think>\nHello there"), "Hello there");
    assert_eq!(clean_reply("  plain  "), "plain");
    assert_eq!(clean_reply("```\ntext\n```"), "text");
}

#[test]
fn check_edit_flags_invented_numbers_but_not_request_ones() {
    let old = "Led a team at Acme.";
    assert!(check_edit(old, "mention 12 people", "Led a team of 12 at Acme.").is_empty());
    let flags = check_edit(old, "make it stronger", "Led a team of 40 at Acme.");
    assert!(flags.iter().any(|f| f.text == "40"));
}
