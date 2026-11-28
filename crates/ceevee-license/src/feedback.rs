//! "Send feedback": posts the user's rating and words to the High Turtle feedback service
//! (`https://feedback.turtlebox.studio/v1/feedback`, see docs/FEEDBACK_API.md).
//!
//! There is no secret in the app. Each install makes its own Ed25519 key the first time it sends
//! feedback and keeps it in the OS keychain. Every request carries `x-ht-install` (the public key,
//! hex), `x-ht-timestamp` (unix seconds) and `x-ht-signature` (hex Ed25519 signature over
//! [`signing_input`]). The signature shows that the request came from the holder of that install
//! key and was not altered; the service rate-limits per install and per address.

use ed25519_dalek::{Signer, SigningKey};

pub const FEEDBACK_URL: &str = "https://feedback.turtlebox.studio/v1/feedback";
pub const APP_ID: &str = "ceevee";

/// Feedback needs no build-time secret, so every build can send it.
pub fn available() -> bool {
    true
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The exact text that is signed: app id, install id, timestamp and the raw body, so a signature
/// cannot be reused for another app, another install, another moment or another message.
pub fn signing_input(install: &str, timestamp: i64, body: &str) -> String {
    format!("{APP_ID}.{install}.{timestamp}.{body}")
}

/// `(install id, signature)` for one request, both hex.
pub fn sign(key: &SigningKey, timestamp: i64, body: &str) -> (String, String) {
    let install = hex(key.verifying_key().as_bytes());
    let sig = key.sign(signing_input(&install, timestamp, body).as_bytes());
    (install, hex(&sig.to_bytes()))
}

#[cfg(any(windows, target_os = "macos"))]
const KEY_ACCOUNT: &str = "feedback-install-key";

/// This install's key: read from the keychain, or made and saved on first use. Where there is no
/// keychain (or it refuses) a key lasts only until the app closes, which still works but means
/// the service sees a new install next time.
fn install_key() -> SigningKey {
    #[cfg(any(windows, target_os = "macos"))]
    {
        if let Ok(entry) = keyring::Entry::new("studio.turtlebox.ceevee", KEY_ACCOUNT) {
            if let Some(seed) = entry.get_password().ok().and_then(|h| unhex_seed(&h)) {
                return SigningKey::from_bytes(&seed);
            }
            let key = new_key();
            if let Err(e) = entry.set_password(&hex(&key.to_bytes())) {
                tracing::warn!("could not save the feedback key to the keychain: {e}");
            }
            return key;
        }
    }
    new_key()
}

fn new_key() -> SigningKey {
    let mut seed = [0u8; 32];
    // The system random source failing is not recoverable; fall back to a time based seed rather
    // than losing feedback (it only identifies the install to the service).
    if getrandom::fill(&mut seed).is_err() {
        let t = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        seed[..8].copy_from_slice(&t.to_le_bytes());
    }
    SigningKey::from_bytes(&seed)
}

#[allow(dead_code)]
fn unhex_seed(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// A sensible-looking email address, matching the service's own check: one `@`, no spaces, a dotted
/// domain of letters, digits and hyphens, and a top-level domain of at least 2 letters.
pub fn email_ok(e: &str) -> bool {
    let Some((local, domain)) = e.split_once('@') else {
        return false;
    };
    let bad_local = |c: char| c.is_whitespace() || c.is_control() || "@\"<>()[],;:\\".contains(c);
    let label_ok = |l: &&str| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    };
    let labels: Vec<&str> = domain.split('.').collect();
    let tld = labels.last().copied().unwrap_or("");
    e.len() <= 254
        && !local.is_empty()
        && local.len() <= 64
        && !local.chars().any(bad_local)
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && labels.len() >= 2
        && labels.iter().all(label_ok)
        && (2..=63).contains(&tld.len())
        && tld.chars().all(|c| c.is_ascii_alphabetic())
}

/// What the user typed, checked the same way the service checks it. Returns the JSON body to send.
/// With `anonymous`, the email is left out entirely and the service is told not to keep one.
pub fn body(
    rating: Option<u8>,
    message: &str,
    category: Option<&str>,
    email: Option<&str>,
    anonymous: bool,
    app_version: &str,
    platform: &str,
) -> Result<String, String> {
    let message = message.trim();
    if rating.is_some_and(|r| !(1..=5).contains(&r)) {
        return Err("Choose from 1 to 5 stars.".into());
    }
    if rating.is_none() && message.is_empty() {
        return Err("Give a star rating or write a few words.".into());
    }
    if message.chars().count() > 5000 {
        return Err("Please keep it under 5,000 characters.".into());
    }
    let mut v =
        serde_json::json!({ "app": APP_ID, "appVersion": app_version, "platform": platform });
    if let Some(r) = rating {
        v["rating"] = r.into();
    }
    if !message.is_empty() {
        v["message"] = message.into();
    }
    if let Some(c) = category.filter(|c| ["feature", "issue", "praise", "other"].contains(c)) {
        v["category"] = c.into();
    }
    if anonymous {
        v["anonymous"] = true.into();
    } else if let Some(e) = email.map(str::trim).filter(|e| !e.is_empty()) {
        if !email_ok(e) {
            return Err(
                "That email address doesn’t look right. Fix it, clear it, or tick Send anonymously."
                    .into(),
            );
        }
        v["email"] = e.into();
    }
    Ok(v.to_string())
}

/// Signs and sends a body made by [`body`]. Errors are worded for the user.
pub async fn send(payload: String) -> Result<(), String> {
    let ts = chrono::Utc::now().timestamp();
    let (install, sig) = sign(&install_key(), ts, &payload);
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(concat!("CeeVee/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let res = http
        .post(FEEDBACK_URL)
        .header("content-type", "application/json")
        .header("x-ht-app", APP_ID)
        .header("x-ht-install", install)
        .header("x-ht-timestamp", ts.to_string())
        .header("x-ht-signature", sig)
        .body(payload)
        .send()
        .await
        .map_err(|_| {
            "Couldn't reach the feedback service. Check your internet connection and try again."
                .to_string()
        })?;
    match res.status().as_u16() {
        200..=299 => Ok(()),
        429 => Err("You've sent a lot of feedback recently. Please try again later.".into()),
        400 => Err(res
            .text()
            .await
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v["error"].as_str().map(str::to_string))
            .unwrap_or_else(|| "The feedback couldn't be sent.".into())),
        code => Err(format!(
            "The feedback service answered {code}. Please try again later."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    #[test]
    fn a_request_is_signed_by_the_install_key_and_verifies() {
        use ed25519_dalek::{Signature, Verifier};
        let k = key(7);
        let (install, sig) = sign(&k, 1_795_694_400, r#"{"app":"ceevee","rating":5}"#);
        // The install id is the public key, so the service needs no shared secret to check it.
        assert_eq!(install, hex(k.verifying_key().as_bytes()));
        let sig_bytes: [u8; 64] = (0..64)
            .map(|i| u8::from_str_radix(&sig[i * 2..i * 2 + 2], 16).unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let msg = signing_input(&install, 1_795_694_400, r#"{"app":"ceevee","rating":5}"#);
        assert!(k
            .verifying_key()
            .verify(msg.as_bytes(), &Signature::from_bytes(&sig_bytes))
            .is_ok());
        // Any change to the body, time or install breaks it.
        for other in [
            signing_input(&install, 1_795_694_401, r#"{"app":"ceevee","rating":5}"#),
            signing_input(&install, 1_795_694_400, r#"{"app":"ceevee","rating":1}"#),
            signing_input("other", 1_795_694_400, r#"{"app":"ceevee","rating":5}"#),
        ] {
            assert!(k
                .verifying_key()
                .verify(other.as_bytes(), &Signature::from_bytes(&sig_bytes))
                .is_err());
        }
    }

    #[test]
    fn installs_have_different_ids_and_signing_is_deterministic() {
        let (a, sa) = sign(&key(1), 10, "{}");
        let (b, _) = sign(&key(2), 10, "{}");
        assert_ne!(a, b);
        assert_eq!(sign(&key(1), 10, "{}"), (a, sa));
        assert!(available());
    }

    #[test]
    fn saved_keys_round_trip_and_junk_is_ignored() {
        let k = key(9);
        assert_eq!(unhex_seed(&hex(&k.to_bytes())), Some(k.to_bytes()));
        assert_eq!(unhex_seed("nope"), None);
        assert_eq!(unhex_seed(&"zz".repeat(32)), None);
    }

    #[test]
    fn body_rules() {
        assert!(body(None, "  ", None, None, false, "1.0", "macos").is_err());
        assert!(body(Some(0), "", None, None, false, "1.0", "macos").is_err());
        assert!(body(Some(6), "", None, None, false, "1.0", "macos").is_err());
        assert!(body(None, &"x".repeat(5001), None, None, false, "1.0", "macos").is_err());
        let v: serde_json::Value = serde_json::from_str(
            &body(
                Some(4),
                " Better cover letters ",
                Some("feature"),
                Some(" "),
                false,
                "1.0",
                "macos",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!({"app":"ceevee","rating":4,"message":"Better cover letters","category":"feature","appVersion":"1.0","platform":"macos"})
        );
        let v: serde_json::Value = serde_json::from_str(
            &body(
                None,
                "hi",
                Some("bogus"),
                Some("a@b.co"),
                false,
                "1.0",
                "linux",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(v["email"], "a@b.co");
        assert!(v.get("category").is_none());
    }

    #[test]
    fn email_and_anonymous() {
        for good in [
            "a@b.co",
            "first.last+tag@mail.example.com",
            "x_y-z@sub-domain.example.io",
        ] {
            assert!(email_ok(good), "{good}");
        }
        for bad in [
            "nope",
            "a@b",
            "a@b.c",
            "a b@c.co",
            "a@b..co",
            ".a@b.co",
            "a..b@c.co",
            "a@-b.co",
            "<a@b.co>",
            "a@b@c.co",
            "a@b.c0",
        ] {
            assert!(!email_ok(bad), "{bad}");
        }
        assert!(body(None, "hi", None, Some("nope"), false, "1.0", "macos").is_err());
        // Anonymous: the address isn't checked or sent, and the service is told so.
        let v: serde_json::Value = serde_json::from_str(
            &body(
                None,
                "hi",
                None,
                Some("me@example.com"),
                true,
                "1.0",
                "macos",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(v.get("email").is_none());
        assert_eq!(v["anonymous"], true);
        assert!(body(None, "hi", None, Some("nope"), true, "1.0", "macos").is_ok());
        // Not anonymous: no flag at all, the same body older builds sent.
        let v: serde_json::Value = serde_json::from_str(
            &body(
                None,
                "hi",
                None,
                Some("me@example.com"),
                false,
                "1.0",
                "macos",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(v["email"], "me@example.com");
        assert!(v.get("anonymous").is_none());
    }
}
