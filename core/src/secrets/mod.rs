//! Secrets files: the project's `.cactus/secrets.json` and each critter's
//! `.cactus/critters/<id>/secrets.json`. They, and the context files, are
//! sealed at rest by [`vault`].

mod store;
mod totp;
mod vault;

pub use store::{SECRETS_FILE, SecretFile, names, read, write};
pub use totp::TotpSetup;
pub use vault::{
    Unlocked, VaultStatus, disable_totp, enable_totp, set_password, start_totp, status, totp_enabled, unlock, wipe,
};
pub(crate) use vault::{read_json as read_sealed, write_json as write_sealed};
