//! Mail: decoding messages, the no-model candidate filter, labelling, linking to applications,
//! status changes and reply drafts. Fetching (IMAP, Gmail) feeds [`pipeline::process`]
//! (see docs/TECHNICAL_PLAN.md section 11).

pub mod filter;
pub mod imap;
pub mod label;
pub mod link;
pub mod parse;
pub mod pipeline;
pub mod reply;
pub mod secrets;
pub mod status;
pub mod store;
pub mod sync;

pub use filter::{classify, Candidate, Rules};
pub use parse::{parse, ParsedMail};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod imap_tests;
