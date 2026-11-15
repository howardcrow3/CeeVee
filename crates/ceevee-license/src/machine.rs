//! The machine ID: a hash, so the raw hardware ID never leaves the computer.

use sha2::{Digest, Sha256};

const APP_SALT: &str = "ceevee/machine/v1";

/// Hash a raw machine identifier with the app salt (lower-case hex).
pub fn hash_machine_id(raw: &str) -> String {
    let mut h = Sha256::new();
    h.update(APP_SALT.as_bytes());
    h.update([0]);
    h.update(raw.trim().as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// This computer's hashed machine ID. Falls back to a stable placeholder if the
/// OS refuses to give an ID, so the app still runs (the license then simply
/// cannot be moved between computers by ID).
pub fn current() -> String {
    match machine_uid::get() {
        Ok(id) => hash_machine_id(&id),
        Err(e) => {
            tracing::warn!("no machine id available: {e}");
            hash_machine_id("unknown-machine")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_hex_and_does_not_contain_the_input() {
        let a = hash_machine_id("abc-123");
        assert_eq!(a, hash_machine_id("  abc-123\n"));
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!a.contains("abc"));
        assert_ne!(a, hash_machine_id("abc-124"));
    }
}
