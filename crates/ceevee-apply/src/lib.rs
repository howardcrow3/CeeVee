//! Applying to jobs: which applicant tracking system a page belongs to, what a form field is
//! asking for, which controls submit, and which pages automation must never touch
//! (docs/TECHNICAL_PLAN.md section 10). The browser driver comes later; these are the pure rules
//! it uses, so they are fully testable without a browser.

pub mod ats;
pub mod fields;
pub mod guard;

pub use ats::{detect, Ats};
pub use fields::{classify_field, FieldKind};
pub use guard::{check_navigation, is_submit_control, Refusal};
