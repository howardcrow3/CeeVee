//! The blank-field check: one list of every field the form asked for, with its state, so the
//! person sees at a glance what was filled, what needs them and what to look at before submitting.
//! Nothing may be submitted while a required field still needs the person.

use serde::Serialize;

use crate::engine::{FillReport, Filled, Skipped};
use crate::fields::FieldKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Filled from the person's own profile or documents.
    Filled,
    /// Filled, but a sensitive or salary answer the person should read before submitting.
    CheckThis,
    /// A required field CeeVee could not answer. Blocks submitting.
    NeedsYou,
    /// An optional field left blank.
    LeftBlank,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    pub label: String,
    pub state: State,
    /// Where the answer came from, or why there is none, in plain words.
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Checklist {
    /// Needs you first, then check this, then left blank, then filled.
    pub items: Vec<Item>,
    /// False while any required field needs the person.
    pub ready_to_submit: bool,
}

fn source(kind: FieldKind) -> &'static str {
    match kind {
        FieldKind::ResumeUpload => "your resume for this job",
        FieldKind::CoverLetter => "your cover letter for this job",
        FieldKind::Sensitive | FieldKind::Salary => "an answer you saved",
        _ => "your profile",
    }
}

fn filled_item(f: &Filled) -> Item {
    Item {
        label: f.label.clone(),
        state: if f.review {
            State::CheckThis
        } else {
            State::Filled
        },
        detail: format!("From {}", source(f.kind)),
    }
}

fn skipped_item(s: &Skipped) -> Item {
    Item {
        label: s.label.clone(),
        state: if s.required {
            State::NeedsYou
        } else {
            State::LeftBlank
        },
        detail: capitalize(&s.reason),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

impl FillReport {
    /// The check shown before the person submits.
    pub fn checklist(&self) -> Checklist {
        let mut items: Vec<Item> = self
            .filled
            .iter()
            .map(filled_item)
            .chain(self.skipped.iter().map(skipped_item))
            .collect();
        let rank = |s: State| match s {
            State::NeedsYou => 0,
            State::CheckThis => 1,
            State::LeftBlank => 2,
            State::Filled => 3,
        };
        // Stable, so fields keep their order on the form within each group.
        items.sort_by_key(|i| rank(i.state));
        let ready_to_submit = !items.iter().any(|i| i.state == State::NeedsYou);
        Checklist {
            items,
            ready_to_submit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ats::Ats;

    fn report() -> FillReport {
        FillReport {
            ats: Ats::Greenhouse,
            url: "https://boards.greenhouse.io/acme/jobs/1".into(),
            filled: vec![
                Filled {
                    label: "Email".into(),
                    value: "ada@example.com".into(),
                    kind: FieldKind::Email,
                    review: false,
                },
                Filled {
                    label: "Authorized to work".into(),
                    value: "Yes".into(),
                    kind: FieldKind::Sensitive,
                    review: true,
                },
            ],
            skipped: vec![
                Skipped {
                    label: "Website".into(),
                    required: false,
                    reason: "nothing saved for this".into(),
                },
                Skipped {
                    label: "Why us?".into(),
                    required: true,
                    reason: "not recognised".into(),
                },
            ],
        }
    }

    #[test]
    fn groups_by_what_the_person_must_do() {
        let c = report().checklist();
        let states: Vec<State> = c.items.iter().map(|i| i.state).collect();
        assert_eq!(
            states,
            [
                State::NeedsYou,
                State::CheckThis,
                State::LeftBlank,
                State::Filled
            ]
        );
        assert_eq!(c.items[0].label, "Why us?");
        assert_eq!(c.items[0].detail, "Not recognised");
    }

    #[test]
    fn blocks_submitting_while_a_required_field_is_empty() {
        assert!(!report().checklist().ready_to_submit);
        let mut r = report();
        r.skipped.retain(|s| !s.required);
        assert!(r.checklist().ready_to_submit);
    }

    #[test]
    fn says_where_each_answer_came_from() {
        let c = report().checklist();
        let email = c.items.iter().find(|i| i.label == "Email").unwrap();
        assert_eq!(email.detail, "From your profile");
        let auth = c
            .items
            .iter()
            .find(|i| i.label == "Authorized to work")
            .unwrap();
        assert_eq!(auth.detail, "From an answer you saved");
    }

    #[test]
    fn an_empty_form_is_ready() {
        let r = FillReport {
            ats: Ats::Greenhouse,
            url: String::new(),
            filled: vec![],
            skipped: vec![],
        };
        assert!(r.checklist().ready_to_submit);
        assert!(r.checklist().items.is_empty());
    }
}
