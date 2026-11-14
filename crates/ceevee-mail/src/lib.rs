//! Mail: decoding messages, the no-model candidate filter, and (later) IMAP/SMTP, OAuth,
//! labelling, linking and alert-email parsing (see docs/TECHNICAL_PLAN.md section 11).

pub mod filter;
pub mod parse;

pub use filter::{classify, Candidate, Rules};
pub use parse::{parse, ParsedMail};
