//! Runs the engine against a real browser and a local form. Uses `CEEVEE_CHROME` if set, else the
//! installed Chrome or Edge, and skips when there is none.
#![cfg(feature = "chrome")]

use ceevee_apply::chrome::ChromeDriver;
use ceevee_apply::engine::{fill_application, Answers, BrowserDriver};

const FORM: &str = r#"<!doctype html><html><body>
<form id="f" onsubmit="return false">
  <label for="fn">First Name *</label><input id="fn" required>
  <label>Last Name <input id="ln"></label>
  <input id="em" type="email" aria-label="Email">
  <label for="wa">Are you legally authorized to work in the US?</label>
  <select id="wa"><option>Select</option><option>Yes</option><option>No</option></select>
  <label for="cv">Resume/CV</label><input id="cv" type="file">
  <textarea aria-label="Why us?"></textarea>
  <input type="hidden" id="h" value="x">
  <button id="submit">Submit application</button>
</form></body></html>"#;

#[tokio::test]
async fn fills_a_real_form_and_never_submits() {
    let exe = match std::env::var("CEEVEE_CHROME") {
        Ok(p) => std::path::PathBuf::from(p),
        Err(_) => match ceevee_apply::chrome::find_browser() {
            Some(p) => p,
            None => {
                eprintln!("no Chrome or Edge found; skipping");
                return;
            }
        },
    };
    let dir = std::env::temp_dir().join(format!("ceevee-chrome-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let page = dir.join("form.html");
    std::fs::write(&page, FORM).unwrap();
    let resume = dir.join("resume.pdf");
    std::fs::write(&resume, b"%PDF-1.4 test").unwrap();

    let mut d = ChromeDriver::launch(&exe, &dir.join("profile"), true)
        .await
        .unwrap();
    let mut a = Answers {
        first_name: "Ada".into(),
        last_name: "Lovelace".into(),
        email: "ada@example.com".into(),
        resume_pdf: Some(resume.display().to_string()),
        ..Answers::default()
    };
    a.saved.insert("authorized to work".into(), "Yes".into());
    let url = url::Url::from_file_path(&page).unwrap().to_string();
    let report = fill_application(&mut d, &url, &a).await.unwrap();

    assert_eq!(report.filled.len(), 5, "{report:?}");
    assert_eq!(
        report.skipped.len(),
        1,
        "the free-text question is left alone"
    );
    // Read the values back out of the page.
    let values = d.page_text().await.unwrap();
    assert!(values.contains("First Name"));
    let json = d
        .evaluate_for_test(
            "JSON.stringify([fn.value, ln.value, em.value, wa.value, cv.files.length])",
        )
        .await
        .unwrap();
    assert_eq!(json, r#"["Ada","Lovelace","ada@example.com","Yes",1]"#);
    d.close().await.unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
