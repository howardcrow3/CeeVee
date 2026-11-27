//! "Send feedback": posts the user's rating and words to the High Turtle feedback service
//! (`https://feedback.turtlebox.studio/v1/feedback`, see howardcrow3/highturtle `feedback/README.md`).
//!
//! Each request carries `x-ht-app`, `x-ht-timestamp` (unix seconds) and `x-ht-signature`: hex
//! HMAC-SHA256 with this app's key over `"<timestamp>.<raw body>"`. The key is compiled in from the
//! `FEEDBACK_KEY` build variable (a GitHub Actions secret) and is never committed.

use sha2::{Digest, Sha256};

pub const FEEDBACK_URL: &str = "https://feedback.turtlebox.studio/v1/feedback";
pub const APP_ID: &str = "ceevee";

/// Baked in at build time. Local builds without it hide the feedback form.
pub const KEY: Option<&str> = option_env!("FEEDBACK_KEY");

pub fn available() -> bool {
    KEY.is_some_and(|k| !k.trim().is_empty())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Plain HMAC-SHA256 (RFC 2104), hex encoded.
pub fn hmac_sha256_hex(key: &[u8], msg: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(k.map(|b| b ^ 0x36));
    inner.update(msg);
    let mut outer = Sha256::new();
    outer.update(k.map(|b| b ^ 0x5c));
    outer.update(inner.finalize());
    hex(&outer.finalize())
}

/// The signature header value for one request. `key_hex` is the app key as stored by the service.
pub fn sign(key_hex: &str, timestamp: i64, body: &str) -> Option<String> {
    Some(hmac_sha256_hex(
        &unhex(key_hex)?,
        format!("{timestamp}.{body}").as_bytes(),
    ))
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
    let key = KEY
        .filter(|k| !k.trim().is_empty())
        .ok_or("Feedback isn't set up in this build of CeeVee.")?;
    let ts = chrono::Utc::now().timestamp();
    let sig = sign(key, ts, &payload)
        .ok_or("Feedback isn't set up correctly in this build of CeeVee.")?;
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(concat!("CeeVee/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let res = http
        .post(FEEDBACK_URL)
        .header("content-type", "application/json")
        .header("x-ht-app", APP_ID)
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

    #[test]
    fn hmac_matches_rfc4231() {
        assert_eq!(
            hmac_sha256_hex(b"Jefe", b"what do ya want for nothing?"),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(
            hmac_sha256_hex(
                &[0xaa; 131],
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            ),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn signature_matches_the_service() {
        // Value computed by the service's own signer (feedback/src/core.mjs signBody) for these inputs.
        let sig = sign(
            &"ab".repeat(32),
            1_795_694_400,
            r#"{"app":"backer","rating":5}"#,
        )
        .unwrap();
        assert_eq!(
            sig,
            "d2523b2c438bb71b1a763eaf45b3eeea2041afaf961bdf15c202c045455d54df"
        );
        assert!(sign("not hex", 1, "{}").is_none());
        assert!(sign("abc", 1, "{}").is_none());
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
