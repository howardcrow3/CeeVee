//! The local connection between the desktop app and the companion browser extension.
//!
//! The extension fills forms in the person's own browser, but everything it needs (the profile,
//! saved answers, documents, the never-list and the blank-field check) stays in the desktop app.
//! This crate is the small web server on `127.0.0.1` that answers it. It never talks to the
//! internet. See `docs/COMPETITIVE_BUILD_PLAN.md`, item 5.
//!
//! Who may call it:
//! - Only this computer: it listens on the loopback address and refuses a `Host` header that is
//!   not that address, which stops a web page reaching it by DNS rebinding.
//! - Only browser extensions: a request with an `Origin` that is a web page is refused.
//! - Only paired extensions: apart from `/ping` and the pairing calls, every request needs the
//!   token the person approved in the app.

pub mod protocol;
mod server;

pub use server::{Backend, BoxFuture, Bridge, PendingPair};
