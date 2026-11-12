use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ceevee_ai::{GenerateRequest, LlmBackend};
use serde_json::json;

use super::*;
use crate::store::{create, current, DocKind};

struct Scripted(Arc<Mutex<Vec<String>>>, bool);
impl LlmBackend for Scripted {
    fn load(&mut self) -> Result<()> {
        self.1 = true;
        Ok(())
    }
    fn unload(&mut self) {
        self.1 = false;
    }
    fn is_loaded(&self) -> bool {
        self.1
    }
    fn generate(
        &mut self,
        _: &GenerateRequest,
        _: &AtomicBool,
        _: &mut dyn FnMut(&str),
    ) -> Result<String> {
        Ok(self.0.lock().unwrap().remove(0))
    }
}

fn worker(replies: Vec<String>) -> ModelWorker {
    ModelWorker::spawn(
        Scripted(Arc::new(Mutex::new(replies)), false),
        Duration::from_secs(60),
    )
}

const PROFILE: &str = "Maria Lopez. Senior Analyst at Northfield Logistics since 2021. Built Tableau dashboards for 40 depots. SQL.";
const POSTING: &str = "Acme Analytics seeks a Data Analyst. SQL and Tableau. 2+ years.";

fn letter_reply(body: &str) -> String {
    json!({ "letter": body }).to_string()
}

const GOOD_LETTER: &str = "Dear Hiring Team,\n\nI am writing about the Data Analyst role at Acme Analytics. At Northfield Logistics I built Tableau dashboards for 40 depots and wrote SQL every week, which is the work your posting describes.\n\nI would bring that same care to your reporting, and I would value the chance to talk about it.\n\nSincerely,\nMaria Lopez";

#[tokio::test]
async fn a_faithful_letter_has_no_flags_and_saves() {
    let w = worker(vec![letter_reply(GOOD_LETTER)]);
    let d = draft_cover_letter(&w, PROFILE, POSTING, "").await.unwrap();
    assert!(d.flags.is_empty(), "{:?}", d.flags);
    assert_eq!(d.created_with, "write_cover_letter@1");

    let db = Db::open_in_memory().await.unwrap();
    let doc = create(&db, DocKind::CoverLetter, "Acme", None, None, "", "created")
        .await
        .unwrap();
    save_draft(&db, doc, &d, &[], "drafted").await.unwrap();
    let v = current(&db, doc).await.unwrap();
    assert_eq!(v.content, GOOD_LETTER);
    assert_eq!(v.created_with.as_deref(), Some("write_cover_letter@1"));
}

#[tokio::test]
async fn an_invented_claim_blocks_saving_until_accepted() {
    let bad = GOOD_LETTER.replace("40 depots", "75 depots and the Globex account");
    let w = worker(vec![letter_reply(&bad)]);
    let d = draft_cover_letter(&w, PROFILE, POSTING, "make it warmer")
        .await
        .unwrap();
    let flagged: Vec<&str> = d.flags.iter().map(|f| f.text.as_str()).collect();
    assert_eq!(flagged, vec!["75", "Globex"]);

    let db = Db::open_in_memory().await.unwrap();
    let doc = create(&db, DocKind::CoverLetter, "Acme", None, None, "", "created")
        .await
        .unwrap();
    let err = save_draft(&db, doc, &d, &[], "drafted").await.unwrap_err();
    assert!(
        err.to_string().contains("\"75\"") && err.to_string().contains("\"Globex\""),
        "{err}"
    );
    assert_eq!(current(&db, doc).await.unwrap().content, "");

    // Accepting only one of them is not enough.
    assert!(save_draft(&db, doc, &d, &["75".into()], "drafted")
        .await
        .is_err());
    save_draft(&db, doc, &d, &["75".into(), "globex".into()], "drafted")
        .await
        .unwrap();
    assert!(current(&db, doc).await.unwrap().content.contains("Globex"));
}

#[tokio::test]
async fn instructions_reach_the_prompt() {
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    struct Spy(Arc<Mutex<Vec<String>>>, bool);
    impl LlmBackend for Spy {
        fn load(&mut self) -> Result<()> {
            self.1 = true;
            Ok(())
        }
        fn unload(&mut self) {
            self.1 = false;
        }
        fn is_loaded(&self) -> bool {
            self.1
        }
        fn generate(
            &mut self,
            r: &GenerateRequest,
            _: &AtomicBool,
            _: &mut dyn FnMut(&str),
        ) -> Result<String> {
            self.0.lock().unwrap().push(r.prompt.clone());
            Ok(json!({"letter": GOOD_LETTER}).to_string())
        }
    }
    let w = ModelWorker::spawn(Spy(seen.clone(), false), Duration::from_secs(60));
    draft_cover_letter(&w, PROFILE, POSTING, "Keep it under 200 words")
        .await
        .unwrap();
    assert!(seen.lock().unwrap()[0].contains("- Also: Keep it under 200 words"));
}

fn base_resume() -> Value {
    json!({
        "name": "Maria Lopez", "email": null, "phone": null, "location": null, "summary": null,
        "work": [{"company": "Northfield Logistics", "title": "Senior Analyst", "start": "2021-03", "end": null, "current": true,
                  "highlights": ["Built Tableau dashboards for 40 depots", "Wrote SQL reports"]}],
        "education": [], "skills": ["Excel", "SQL", "Tableau"]
    })
}

#[tokio::test]
async fn tailoring_that_only_reorders_passes() {
    let mut tailored = base_resume();
    tailored["skills"] = json!(["SQL", "Tableau"]);
    let w = worker(vec![tailored.to_string()]);
    let d = draft_tailored_resume(&w, &base_resume(), POSTING)
        .await
        .unwrap();
    assert!(d.flags.is_empty(), "{:?}", d.flags);
    assert_eq!(d.created_with, "tailor_resume@1");
}

#[tokio::test]
async fn tailoring_cannot_borrow_numbers_from_the_posting() {
    let mut tailored = base_resume();
    tailored["work"][0]["highlights"] =
        json!(["Built Tableau dashboards", "Wrote SQL reports for 2 years"]);
    tailored["skills"] = json!(["SQL", "Tableau", "Kubernetes"]);
    let w = worker(vec![tailored.to_string()]);
    let d = draft_tailored_resume(&w, &base_resume(), POSTING)
        .await
        .unwrap();
    let mut flagged: Vec<&str> = d.flags.iter().map(|f| f.text.as_str()).collect();
    flagged.sort_unstable();
    assert_eq!(flagged, vec!["2", "Kubernetes"]);
}

#[test]
fn rechecking_edited_text_uses_the_same_rules() {
    assert!(recheck_letter(GOOD_LETTER, PROFILE, POSTING).is_empty());
    assert_eq!(
        recheck_letter("I led 12 people.", PROFILE, POSTING)[0].text,
        "12"
    );
    assert!(recheck_resume(&base_resume(), &crate::fact_guard::text_of(&base_resume())).is_empty());
}
