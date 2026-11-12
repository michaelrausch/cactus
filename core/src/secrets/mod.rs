//! Secret encryption: opt-in per project, per machine. Each value in a
//! secrets file is sealed in place (`"API_KEY": "enc:v1:<salt>:<…>"`);
//! names stay readable, and an older cactus still reads the file. Whether
//! it's on, and the auto-lock setting, live in `.cactus/state.json`. See
//! `docs/secrets-encryption-plan.md`.
//!
//! Built only with the `encryption` feature. Without it, secrets are always
//! plain text, the same functions exist and refuse with
//! [`Error::EncryptionOff`](crate::Error::EncryptionOff), and a sealed value
//! is never used as a literal.

#[cfg(feature = "encryption")]
mod cipher;
#[cfg(not(feature = "encryption"))]
mod disabled;
#[cfg(feature = "encryption")]
mod keyring;
#[cfg(feature = "encryption")]
mod lifecycle;
mod store;
#[cfg(all(test, feature = "encryption"))]
mod tests;

use serde::{Deserialize, Serialize};

#[cfg(feature = "encryption")]
pub use cipher::{MIN_PASSWORD_CHARS, SEALED_PREFIX, SecretString};
#[cfg(not(feature = "encryption"))]
pub use disabled::{
    SecretString, change_password, disable, enable, has_sealed_values, info, lock, lock_all, reset, set_auto_lock,
    status, unlock, unlock_read_only,
};
#[cfg(feature = "encryption")]
pub use lifecycle::{
    change_password, disable, enable, has_sealed_values, info, lock, lock_all, reset, set_auto_lock, status, unlock,
    unlock_read_only,
};
pub use store::{SECRETS_FILE, SecretFile, locked_for, names, read, write};

pub use crate::state::DEFAULT_AUTO_LOCK_MINUTES;

/// Whether this build can encrypt secrets.
pub const ENABLED: bool = cfg!(feature = "encryption");

/// What every encrypted value starts with, whatever its version.
const ENCRYPTED_PREFIX: &str = "enc:";

/// Whether `value` is encrypted rather than plain text.
pub fn is_sealed(value: &str) -> bool {
    value.starts_with(ENCRYPTED_PREFIX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum SecretsStatus {
    /// Secrets are stored as plain text, as before.
    Off,
    /// On, and this process doesn't hold the key for every sealed value.
    Locked,
    Unlocked,
}

/// What the UI shows about encryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SecretsInfo {
    pub status: SecretsStatus,
    /// Lock after this many idle minutes; `None` is never.
    pub auto_lock_minutes: Option<u32>,
    /// Whether any value is sealed. Without one, unlocking can't check the
    /// password, so it sets it instead.
    pub has_sealed_values: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UnlockReport {
    /// Values that were stored as plain text and are now sealed.
    pub sealed_plain_values: u32,
}
