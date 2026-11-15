//! The signed envelope: `base64url(json).base64url(ed25519 signature of the json bytes)`.
//!
//! Licenses and trial tokens share it. A `kind` tag in the signed JSON stops a
//! trial token being passed off as a license.

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier as _, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::{LicenseError, Result};

/// What turtlebox.studio signs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Claims {
    /// A purchased license for one computer (one of the 2 seats).
    License {
        key: String,
        /// Hashed machine ID the seat was issued to.
        machine: String,
        issued_at: DateTime<Utc>,
        /// Highest app major version this license covers (updates are free within it).
        major: u32,
        /// Seats the license allows, for display.
        seats: u32,
    },
    /// The first time the server saw this computer.
    Trial {
        machine: String,
        started_at: DateTime<Utc>,
    },
}

/// Checks signatures against the public key built into the app.
#[derive(Debug, Clone)]
pub struct Verifier {
    key: VerifyingKey,
}

impl Verifier {
    pub fn new(key: VerifyingKey) -> Self {
        Self { key }
    }

    /// From a 64-character hex public key.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim();
        if hex.len() != 64 || !hex.is_ascii() {
            return None;
        }
        let mut bytes = [0u8; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
        }
        VerifyingKey::from_bytes(&bytes).ok().map(Self::new)
    }

    /// The key compiled into this build (`CEEVEE_LICENSE_PUBKEY`, set by the release
    /// build). With no key, every check fails closed and no license can unlock the app.
    pub fn compiled() -> Option<Self> {
        option_env!("CEEVEE_LICENSE_PUBKEY").and_then(Self::from_hex)
    }

    /// Verify a token and return its claims.
    pub fn verify(&self, token: &str) -> Result<Claims> {
        let (payload, sig) = token
            .trim()
            .split_once('.')
            .ok_or(LicenseError::BadSignature)?;
        let payload = B64
            .decode(payload)
            .map_err(|_| LicenseError::BadSignature)?;
        let sig = B64.decode(sig).map_err(|_| LicenseError::BadSignature)?;
        let sig = Signature::from_slice(&sig).map_err(|_| LicenseError::BadSignature)?;
        self.key
            .verify(&payload, &sig)
            .map_err(|_| LicenseError::BadSignature)?;
        serde_json::from_slice(&payload).map_err(|_| LicenseError::BadSignature)
    }
}

/// Sign claims. The app never signs; this is for the turtlebox.studio backend to
/// match (the highturtle repo) and for tests.
pub fn sign(key: &SigningKey, claims: &Claims) -> String {
    let payload = serde_json::to_vec(claims).expect("claims serialise");
    let sig = key.sign(&payload);
    format!("{}.{}", B64.encode(payload), B64.encode(sig.to_bytes()))
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    pub fn keypair() -> (SigningKey, Verifier) {
        let sk = SigningKey::generate(&mut rand_core::OsRng);
        let v = Verifier::new(sk.verifying_key());
        (sk, v)
    }
}

#[cfg(test)]
mod tests {
    use super::testing::keypair;
    use super::*;

    fn lic() -> Claims {
        Claims::License {
            key: "CEEV-AAAAA-BBBBB-CCCCC-DDDDD".into(),
            machine: "m".into(),
            issued_at: "2026-12-01T00:00:00Z".parse().unwrap(),
            major: 1,
            seats: 2,
        }
    }

    #[test]
    fn round_trip() {
        let (sk, v) = keypair();
        assert_eq!(v.verify(&sign(&sk, &lic())).unwrap(), lic());
    }

    #[test]
    fn rejects_tampering_wrong_key_and_junk() {
        let (sk, v) = keypair();
        let t = sign(&sk, &lic());
        let (p, s) = t.split_once('.').unwrap();
        // A different payload with the old signature.
        let other = B64.encode(
            serde_json::to_vec(&Claims::License {
                key: "CEEV-ZZZZZ-BBBBB-CCCCC-DDDDD".into(),
                machine: "m".into(),
                issued_at: "2026-12-01T00:00:00Z".parse().unwrap(),
                major: 9,
                seats: 2,
            })
            .unwrap(),
        );
        assert!(v.verify(&format!("{other}.{s}")).is_err());
        let _ = p;
        let (_, v2) = keypair();
        assert!(v2.verify(&t).is_err());
        for junk in ["", "abc", "a.b", "....", "!!.!!"] {
            assert!(v.verify(junk).is_err(), "{junk}");
        }
    }

    #[test]
    fn the_release_public_key_is_a_valid_ed25519_key() {
        let hex = "425825fea3f127966fe6038b40d17779aac03560a6c9501fb95ba0d55435472a";
        assert!(Verifier::from_hex(hex).is_some());
        let workflow = include_str!("../../../.github/workflows/release.yml");
        assert!(
            workflow.contains(hex),
            "release.yml must carry the same public key"
        );
    }

    #[test]
    fn hex_key_parsing() {
        let (sk, _) = keypair();
        let hex: String = sk
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let v = Verifier::from_hex(&hex).unwrap();
        assert!(v.verify(&sign(&sk, &lic())).is_ok());
        assert!(Verifier::from_hex("xyz").is_none());
        assert!(Verifier::from_hex(&"0".repeat(63)).is_none());
    }
}
