//! What the extension and the app say to each other.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ping {
    pub app: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairRequest {
    /// Shown to the person when they approve: "Chrome", "Edge", "Firefox".
    pub browser: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairStarted {
    pub id: String,
}

/// `pending`, `approved` or `denied`. The token comes once, with `approved`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairStatus {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub token: Option<String>,
}

/// A control on the page, as the extension reads it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field {
    /// A handle the extension understands (it sets one on each element it reads).
    pub id: String,
    pub label: String,
    /// `text`, `textarea`, `select`, `checkbox` or `file`.
    pub kind: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanRequest {
    pub url: String,
    /// Visible text of the page, used to spot CAPTCHA and sign-up pages. Not stored.
    #[serde(default)]
    pub page_text: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Step {
    pub field_id: String,
    /// `fill` or `upload`.
    pub action: String,
    /// The text to put in, or for an upload which document to fetch: `resume` or `cover_letter`.
    pub value: String,
}

/// One line of the blank-field check shown before anything is submitted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckLine {
    pub label: String,
    /// `filled`, `check_this`, `needs_you` or `left_blank`.
    pub state: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PlanReply {
    /// Set when CeeVee never automates this page. Nothing else is filled in then.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub refused: Option<String>,
    pub steps: Vec<Step>,
    pub checklist: Vec<CheckLine>,
    /// False while a required field is empty or needs a look.
    pub ready_to_submit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Document {
    pub file_name: String,
    pub data_base64: String,
}
