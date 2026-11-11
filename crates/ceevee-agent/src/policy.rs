//! The policy engine (TECHNICAL_PLAN section 6.6). Every tool call passes through [`Policy::decide`]
//! before it runs, and the answer is one of run, queue for the user, or deny.
//!
//! Policy comes only from the user's settings. Text inside a posting or an email can never raise a
//! mode: a call whose input came from untrusted content is queued instead of run.

use std::collections::HashMap;

use crate::tools::{Risk, ToolSpec};

/// What the user chose for one action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Off,
    Ask,
    Auto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Run,
    /// Becomes an entry in the approvals queue, with the reason shown to the user.
    Queue(String),
    Deny(String),
}

/// Actions no agent may take at any level, whatever the policy says. A tool whose action starts
/// with one of these (before a dot or at the end) is refused.
pub const NEVER: &[&str] = &[
    "linkedin.message",
    "linkedin.connect",
    "linkedin.easy_apply",
    "apply.one_click",
    "browser.solve_captcha",
    "browser.create_account",
    "data.delete",
    "settings.license",
    "settings.policy",
];

pub fn is_never(action: &str) -> bool {
    NEVER
        .iter()
        .any(|n| action == *n || action.strip_prefix(n).is_some_and(|r| r.starts_with('.')))
}

/// The facts about one call that the engine needs.
#[derive(Debug, Clone, Copy)]
pub struct Call<'a> {
    pub tool: &'a ToolSpec,
    /// The category (search profile) the call concerns, for per-category overrides.
    pub category: Option<i64>,
    /// The user started this exact action, for example by clicking Apply.
    pub user_initiated: bool,
    /// Some of the call's input came from a posting, email or web page.
    pub from_untrusted: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Policy {
    global: HashMap<String, Mode>,
    category: HashMap<(i64, String), Mode>,
}

impl Policy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_global(&mut self, action: &str, mode: Mode) {
        self.global.insert(action.to_owned(), mode);
    }

    pub fn set_category(&mut self, category: i64, action: &str, mode: Mode) {
        self.category.insert((category, action.to_owned()), mode);
    }

    /// Category rows override global rows. With no row, the answer is `Ask`.
    pub fn mode(&self, action: &str, category: Option<i64>) -> Mode {
        category
            .and_then(|c| self.category.get(&(c, action.to_owned())))
            .or_else(|| self.global.get(action))
            .copied()
            .unwrap_or(Mode::Ask)
    }

    pub fn decide(&self, call: &Call<'_>) -> Decision {
        let tool = call.tool;
        if let Some(action) = tool.action {
            if is_never(action) {
                return Decision::Deny(format!("{action} is never available to an agent"));
            }
        }
        if is_never(tool.name) {
            return Decision::Deny(format!("{} is never available to an agent", tool.name));
        }
        match tool.risk {
            Risk::Read | Risk::Draft => Decision::Run,
            Risk::Propose => Decision::Queue("a proposal for you to accept or edit".into()),
            Risk::Act => {
                if call.user_initiated {
                    return Decision::Run;
                }
                let action = tool.action.unwrap_or(tool.name);
                match self.mode(action, call.category) {
                    Mode::Off => Decision::Deny(format!("{action} is turned off")),
                    Mode::Ask => Decision::Queue(format!("{action} needs your approval")),
                    Mode::Auto if call.from_untrusted => Decision::Queue(
                        "this step used text from a posting, email or web page, so it needs your approval"
                            .into(),
                    ),
                    Mode::Auto => Decision::Run,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
