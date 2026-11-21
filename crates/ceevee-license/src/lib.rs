//! Trial, activation and offline Ed25519 license checks
//! (docs/PLAN.md "Licensing, trial and distribution", docs/TECHNICAL_PLAN.md section 13).
//!
//! The pieces are small and separate so each can be tested alone:
//! - [`token`]: the signed envelope used for both licenses and trial tokens.
//! - [`status`]: pure rules that turn stored facts into a [`Status`].
//! - [`key`]: license key shape, and the `ceevee://activate` link.
//! - [`api`]: the turtlebox.studio client (interface documented in docs/LICENSE_API.md).
//! - [`anchor`]: the second place the trial start is kept, the OS keychain.
//! - [`service`]: ties them to the database.

pub mod anchor;
pub mod api;
pub mod key;
pub mod machine;
pub mod service;
pub mod status;
pub mod token;

pub use anchor::{Anchor, MemoryAnchor};
pub use api::{Api, HttpApi};
pub use service::LicenseService;
pub use status::{Status, TRIAL_DAYS};
pub use token::{Claims, Verifier};

use ceevee_core::Error as CoreError;

/// Everything that can go wrong with a license, with wording fit to show a person.
#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("That does not look like a CeeVee license key. It starts with CEEV and has four groups of letters and numbers.")]
    BadKeyFormat,
    #[error("The license from turtlebox.studio could not be verified. Please try again or contact support.")]
    BadSignature,
    #[error("That license belongs to a different computer.")]
    WrongMachine,
    #[error("That license is for a different version of CeeVee.")]
    WrongVersion,
    #[error("This license is already active on 2 computers. Free a seat on turtlebox.studio, then try again.")]
    SeatsFull,
    #[error("turtlebox.studio does not recognise that key. Check it and try again.")]
    UnknownKey,
    #[error("Could not reach turtlebox.studio. Check your internet connection and try again.")]
    Network,
    #[error("turtlebox.studio said no: {0}")]
    Rejected(String),
    #[error("Your 14-day trial has ended, so CeeVee is read-only. You can still view and export everything. Activate a license in Settings to keep going.")]
    ReadOnly,
    #[error("License checks are not available in this build.")]
    NoPublicKey,
    #[error(transparent)]
    Core(#[from] CoreError),
}

pub type Result<T> = std::result::Result<T, LicenseError>;
