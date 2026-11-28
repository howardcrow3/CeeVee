use super::*;

#[derive(Default)]
struct Scripted {
    url: String,
    text: String,
    fields: Vec<FormField>,
    filled: Vec<(String, String)>,
    uploaded: Vec<(String, String)>,
    visited: Vec<String>,
}

impl BrowserDriver for Scripted {
    async fn goto(&mut self, url: &str) -> Result<()> {
        self.visited.push(url.to_string());
        if self.url.is_empty() {
            self.url = url.to_string();
        }
        Ok(())
    }
    async fn current_url(&mut self) -> Result<String> {
        Ok(self.url.clone())
    }
    async fn page_text(&mut self) -> Result<String> {
        Ok(self.text.clone())
    }
    async fn fields(&mut self) -> Result<Vec<FormField>> {
        Ok(self.fields.clone())
    }
    async fn fill(&mut self, id: &str, value: &str) -> Result<()> {
        self.filled.push((id.into(), value.into()));
        Ok(())
    }
    async fn upload(&mut self, id: &str, path: &str) -> Result<()> {
        self.uploaded.push((id.into(), path.into()));
        Ok(())
    }
}

fn field(id: &str, label: &str, input: Input, required: bool) -> FormField {
    FormField {
        id: id.into(),
        label: label.into(),
        input,
        required,
    }
}

fn answers() -> Answers {
    let mut a = Answers {
        first_name: "Ada".into(),
        last_name: "Lovelace".into(),
        email: "ada@example.com".into(),
        phone: "555-0100".into(),
        resume_pdf: Some("/tmp/resume.pdf".into()),
        ..Answers::default()
    };
    a.saved.insert("authorized to work".into(), "Yes".into());
    a
}

const URL: &str = "https://boards.greenhouse.io/acme/jobs/1";

#[tokio::test]
async fn fills_known_fields_and_uploads_the_resume() {
    let mut d = Scripted {
        fields: vec![
            field("f", "First Name *", Input::Text, true),
            field("l", "Last Name", Input::Text, true),
            field("e", "Email", Input::Text, true),
            field("r", "Resume/CV", Input::File, true),
        ],
        ..Scripted::default()
    };
    let r = fill_application(&mut d, URL, &answers()).await.unwrap();
    assert_eq!(r.ats, Ats::Greenhouse);
    assert_eq!(r.filled.len(), 4);
    assert_eq!(d.filled[0], ("f".into(), "Ada".into()));
    assert_eq!(
        d.uploaded,
        vec![("r".to_string(), "/tmp/resume.pdf".to_string())]
    );
    assert!(r.missing_required().is_empty());
}

#[tokio::test]
async fn sensitive_questions_use_only_saved_answers() {
    let mut d = Scripted {
        fields: vec![
            field(
                "a",
                "Are you legally authorized to work in the US?",
                Input::Text,
                true,
            ),
            field("s", "Will you require visa sponsorship?", Input::Text, true),
            field("g", "Gender identity", Input::Text, false),
            field("pay", "Desired salary", Input::Text, false),
        ],
        ..Scripted::default()
    };
    let r = fill_application(&mut d, URL, &answers()).await.unwrap();
    assert_eq!(r.filled.len(), 1);
    assert!(r.filled[0].review, "sensitive answers are always reviewed");
    assert_eq!(r.skipped.len(), 3);
    assert_eq!(
        r.missing_required(),
        vec!["Will you require visa sponsorship?"]
    );
    assert_eq!(d.filled.len(), 1);
}

#[tokio::test]
async fn unknown_fields_are_left_alone_and_a_missing_document_is_reported() {
    let mut a = answers();
    a.resume_pdf = None;
    let mut d = Scripted {
        fields: vec![
            field("c", "Favourite colour", Input::Text, false),
            field("r", "Resume", Input::File, true),
            field("cl", "Cover letter", Input::File, false),
        ],
        ..Scripted::default()
    };
    let r = fill_application(&mut d, URL, &a).await.unwrap();
    assert!(r.filled.is_empty());
    assert!(d.filled.is_empty() && d.uploaded.is_empty());
    assert_eq!(r.missing_required(), vec!["Resume"]);
}

#[tokio::test]
async fn dropdowns_only_take_a_listed_choice() {
    let mut d = Scripted {
        fields: vec![
            field(
                "w",
                "Are you authorized to work in the United States?",
                Input::Select(vec!["Select".into(), "Yes, I am".into(), "No".into()]),
                true,
            ),
            field(
                "v",
                "Do you require visa sponsorship?",
                Input::Select(vec!["Maybe".into()]),
                true,
            ),
        ],
        ..Scripted::default()
    };
    let mut a = answers();
    a.saved.insert("sponsorship".into(), "No".into());
    let r = fill_application(&mut d, URL, &a).await.unwrap();
    assert_eq!(d.filled, vec![("w".to_string(), "Yes, I am".to_string())]);
    assert_eq!(r.skipped.len(), 1);
}

#[tokio::test]
async fn refuses_the_never_list_before_touching_the_browser() {
    let mut d = Scripted::default();
    let e = fill_application(
        &mut d,
        "https://www.linkedin.com/jobs/view/1/apply/",
        &answers(),
    )
    .await
    .unwrap_err();
    assert!(e.to_string().contains("LinkedIn Easy Apply"));
    assert!(d.visited.is_empty());
}

#[tokio::test]
async fn refuses_a_page_that_turns_out_to_be_a_captcha() {
    let mut d = Scripted {
        text: "Please complete the CAPTCHA to continue".into(),
        fields: vec![field("e", "Email", Input::Text, true)],
        ..Scripted::default()
    };
    let e = fill_application(&mut d, URL, &answers()).await.unwrap_err();
    assert!(e.to_string().contains("CAPTCHA"));
    assert!(d.filled.is_empty());
}

#[tokio::test]
async fn refuses_a_redirect_onto_a_blocked_page() {
    let mut d = Scripted {
        url: "https://www.linkedin.com/messaging/thread/1".into(),
        ..Scripted::default()
    };
    assert!(fill_application(&mut d, URL, &answers()).await.is_err());
}

#[test]
fn answers_from_a_profile() {
    let a = Answers::from_parts(
        "Ada King Lovelace",
        Some(" ada@example.com "),
        None,
        Some("London, UK"),
        &[
            "https://www.linkedin.com/in/ada".to_string(),
            "https://ada.example".to_string(),
        ],
        &[("Authorized To Work".to_string(), "Yes".to_string())],
    );
    assert_eq!(
        (a.first_name.as_str(), a.last_name.as_str()),
        ("Ada", "King Lovelace")
    );
    assert_eq!(a.email, "ada@example.com");
    assert_eq!(a.city, "London");
    assert!(a.linkedin.contains("linkedin.com/in/ada"));
    assert_eq!(a.website, "https://ada.example");
    assert_eq!(
        a.saved_for("Are you authorized to work in the US?"),
        Some("Yes")
    );
}

#[test]
fn a_single_name_is_a_first_name_only() {
    let a = Answers::from_parts("Cher", None, None, None, &[], &[]);
    assert_eq!((a.first_name.as_str(), a.last_name.as_str()), ("Cher", ""));
}

#[test]
fn a_plan_needs_no_browser_and_leaves_sensitive_and_unknown_fields_alone() {
    let fields = vec![
        field("e", "Email", Input::Text, true),
        field("r", "Resume/CV", Input::File, true),
        field(
            "w",
            "Are you authorized to work in the US?",
            Input::Text,
            true,
        ),
        field("s", "Will you require sponsorship?", Input::Text, true),
        field("q", "Favourite colour", Input::Text, false),
    ];
    let p = plan_fields(&fields, &answers());
    assert_eq!(
        p.steps,
        vec![
            Step {
                field_id: "e".into(),
                action: Action::Fill("ada@example.com".into())
            },
            Step {
                field_id: "r".into(),
                action: Action::Upload("/tmp/resume.pdf".into())
            },
            Step {
                field_id: "w".into(),
                action: Action::Fill("Yes".into())
            },
        ]
    );
    // The saved answer is shown for review; the unanswered ones are left for the person.
    assert!(p
        .filled
        .iter()
        .any(|f| f.label.contains("authorized") && f.review));
    assert_eq!(
        p.skipped
            .iter()
            .map(|s| s.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Will you require sponsorship?", "Favourite colour"]
    );
}
