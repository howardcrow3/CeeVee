//! Saving a Google account once the browser sign-in has finished.

use ceevee_db::Db;
use ceevee_google::oauth::{Tokens, GMAIL_READONLY};

use crate::secrets::SecretStore;
use crate::store::{self, Account};

/// The keychain key for a Google account's refresh token.
pub fn google_key(address: &str) -> String {
    format!("google:{address}")
}

/// Check what Google granted, put the refresh token in `secrets`, and record the account so
/// Settings shows it as signed in. Errors are written for the person to read.
pub async fn save_google_account(
    db: &Db,
    secrets: &dyn SecretStore,
    tokens: &Tokens,
    address: &str,
) -> Result<Account, String> {
    if !tokens.scopes.iter().any(|s| s == GMAIL_READONLY) {
        return Err("Google did not give CeeVee permission to read your mail. Try again and leave the Gmail box ticked.".into());
    }
    let refresh = tokens
        .refresh_token
        .as_deref()
        .ok_or("Google did not return a long-lasting sign-in. Remove CeeVee at myaccount.google.com/permissions and try again.")?;
    let address = address.trim().to_ascii_lowercase();
    let key = google_key(&address);
    // Replace any earlier item outright. An item left by another build of the app (every test
    // build is a different app to macOS) can refuse an update but still be replaced, and the
    // new item then belongs to this build.
    secrets.delete(&key);
    secrets.set(&key, refresh)?;
    // Read it back: a keychain that accepts a write but returns nothing would otherwise look
    // signed in until the first mail check fails.
    if secrets.get(&key).as_deref() != Some(refresh) {
        return Err("CeeVee could not read back the sign-in it saved to the keychain. Choose Always Allow if macOS asked, then try again.".into());
    }
    store::add_account(db, "gmail", &address, &serde_json::json!({}), &key)
        .await
        .map_err(|e| e.to_string())
}

/// Sign out: forget the account and its mail, and delete the saved token. Returns the saved token
/// so the caller can also revoke it with Google. The local sign-out never depends on the network.
pub async fn forget_account(
    db: &Db,
    secrets: &dyn SecretStore,
    id: i64,
) -> Result<Option<String>, String> {
    let key = store::secret_ref(db, id).await.map_err(|e| e.to_string())?;
    let token = key.as_deref().and_then(|k| secrets.get(k));
    if let Some(k) = &key {
        secrets.delete(k);
    }
    store::remove_account(db, id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(token)
}
