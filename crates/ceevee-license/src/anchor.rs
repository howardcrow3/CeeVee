//! A second home for the trial start, outside the database, so deleting the
//! database does not restart the trial. In the app this is the OS keychain.

use std::sync::Mutex;

use chrono::{DateTime, Utc};

pub trait Anchor: Send + Sync {
    fn get(&self) -> Option<DateTime<Utc>>;
    fn set(&self, start: DateTime<Utc>);
}

/// Test double and fallback when no keychain exists.
#[derive(Default)]
pub struct MemoryAnchor(Mutex<Option<DateTime<Utc>>>);

impl MemoryAnchor {
    pub fn with(start: DateTime<Utc>) -> Self {
        Self(Mutex::new(Some(start)))
    }
}

impl Anchor for MemoryAnchor {
    fn get(&self) -> Option<DateTime<Utc>> {
        *self.0.lock().unwrap()
    }
    fn set(&self, start: DateTime<Utc>) {
        *self.0.lock().unwrap() = Some(start);
    }
}

/// Windows Credential Manager and the macOS Keychain.
#[cfg(any(windows, target_os = "macos"))]
pub struct Keychain {
    entry: Option<keyring::Entry>,
}

#[cfg(any(windows, target_os = "macos"))]
impl Keychain {
    pub fn new() -> Self {
        let entry = keyring::Entry::new("studio.turtlebox.ceevee", "trial-start")
            .map_err(|e| tracing::warn!("keychain unavailable: {e}"))
            .ok();
        Self { entry }
    }
}

#[cfg(any(windows, target_os = "macos"))]
impl Default for Keychain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(windows, target_os = "macos"))]
impl Anchor for Keychain {
    fn get(&self) -> Option<DateTime<Utc>> {
        self.entry.as_ref()?.get_password().ok()?.parse().ok()
    }
    fn set(&self, start: DateTime<Utc>) {
        if let Some(e) = &self.entry {
            if let Err(err) = e.set_password(&start.to_rfc3339()) {
                tracing::warn!("could not write trial start to keychain: {err}");
            }
        }
    }
}

/// The anchor for this platform.
pub fn platform() -> Box<dyn Anchor> {
    #[cfg(any(windows, target_os = "macos"))]
    {
        Box::new(Keychain::new())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Box::new(MemoryAnchor::default())
    }
}

impl Anchor for Box<dyn Anchor> {
    fn get(&self) -> Option<DateTime<Utc>> {
        (**self).get()
    }
    fn set(&self, start: DateTime<Utc>) {
        (**self).set(start)
    }
}
