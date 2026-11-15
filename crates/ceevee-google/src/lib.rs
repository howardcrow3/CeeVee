//! Optional Google sign-in and read-only Gmail access (docs/TECHNICAL_PLAN.md section 8.2).
//!
//! - [`oauth`]: OAuth 2 for installed apps. PKCE, a loopback redirect on a random local port, and
//!   refresh tokens that the caller keeps in the operating system keychain.
//! - [`gmail`]: the few Gmail API calls CeeVee needs, behind the [`gmail::MailApi`] trait so the
//!   sync can be tested without a network.
//!
//! The only Gmail scope requested is `gmail.readonly`, a restricted scope. Google must verify the
//! app before public release; until then only listed test users can sign in.

pub mod gmail;
pub mod oauth;

pub use gmail::{Gmail, MailApi};
pub use oauth::{OAuthClient, Tokens};

#[cfg(test)]
mod tests;
