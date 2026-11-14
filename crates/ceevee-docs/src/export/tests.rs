use super::*;

#[test]
fn file_stem_is_safe_and_never_empty() {
    assert_eq!(
        file_stem("Cover letter: Acme/Inc?"),
        "Cover letter Acme Inc"
    );
    assert_eq!(file_stem("../../etc/passwd"), "etc passwd");
    assert_eq!(file_stem("???"), "document");
    assert_eq!(file_stem(&"a".repeat(200)).len(), 80);
}

#[test]
fn letters_render_to_a_pdf() {
    let pdf = pdf_for(
        DocKind::CoverLetter,
        "Dear team,\n\nHello.",
        Paper::UsLetter,
    )
    .unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}

#[test]
fn resumes_render_and_bad_json_is_refused() {
    let ok = pdf_for(
        DocKind::BaseResume,
        r#"{"basics":{"name":"Ada"}}"#,
        Paper::A4,
    )
    .unwrap();
    assert!(ok.starts_with(b"%PDF"));
    assert!(pdf_for(DocKind::BaseResume, "nope", Paper::A4).is_err());
}
