//! The cryptography. A sealed value carries everything needed to open it
//! except the password:
//!
//! ```text
//! enc:v1:<base64url(salt)>:<base64url(nonce ‖ ciphertext ‖ tag)>
//! ```
//!
//! The key is Argon2id(password, salt) with the parameters fixed by `v1`;
//! the cipher is XChaCha20-Poly1305 with a fresh nonce per value, and the
//! variable's name as associated data, so a value can't be moved to another
//! name. A project uses one salt for all its values, so the key is derived
//! once per unlock.

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use zeroize::{Zeroize, Zeroizing};

use crate::{Error, Result};

/// What a value this build can open starts with.
pub const SEALED_PREFIX: &str = "enc:v1:";
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;
const SALT_LEN: usize = 16;
const VALUE_AAD_PREFIX: &str = "cactus/secrets/v1/";
pub const MIN_PASSWORD_CHARS: usize = 8;

/// Argon2id cost for `v1`: 64 MiB and 3 passes, about half a second once
/// per session. Tests use the minimum so they stay fast.
#[cfg(not(test))]
const V1_COST: (u32, u32, u32) = (64 * 1024, 3, 1);
#[cfg(test)]
const V1_COST: (u32, u32, u32) = (8, 1, 1);

/// A password on its way from the user to the KDF. No `Debug`, and wiped on drop.
pub struct SecretString(Zeroizing<String>);

impl SecretString {
    pub fn new(password: String) -> Self {
        Self(Zeroizing::new(password))
    }

    /// Refuse passwords shorter than [`MIN_PASSWORD_CHARS`] characters.
    pub fn check_length(&self) -> Result<()> {
        if self.0.chars().count() < MIN_PASSWORD_CHARS {
            return Err(Error::PasswordTooShort);
        }
        Ok(())
    }
}

impl From<String> for SecretString {
    fn from(password: String) -> Self {
        Self::new(password)
    }
}

/// The salt a project's values share, base64url as it appears in them.
pub type Salt = String;

/// A key derived from the password and a salt. No `Debug`, and wiped on drop.
pub struct DataKey {
    salt: Salt,
    bytes: [u8; KEY_LEN],
}

impl Drop for DataKey {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

impl DataKey {
    /// The key for `password` under a fresh salt.
    pub fn generate(password: &SecretString) -> Result<Self> {
        let salt: [u8; SALT_LEN] = random()?;
        Self::derive(password, &URL_SAFE_NO_PAD.encode(salt))
    }

    /// The key for `password` under `salt`.
    pub fn derive(password: &SecretString, salt: &str) -> Result<Self> {
        let salt_bytes = URL_SAFE_NO_PAD.decode(salt).map_err(|_| Error::SecretCorrupt("bad salt".into()))?;
        let (memory_kib, iterations, parallelism) = V1_COST;
        let params = Params::new(memory_kib, iterations, parallelism, Some(KEY_LEN))
            .map_err(|e| Error::SecretCorrupt(format!("key derivation: {e}")))?;
        let mut key = Self { salt: salt.to_string(), bytes: [0; KEY_LEN] };
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
            .hash_password_into(password.0.as_bytes(), &salt_bytes, &mut key.bytes)
            .map_err(|e| Error::SecretCorrupt(format!("key derivation: {e}")))?;
        Ok(key)
    }

    pub fn salt(&self) -> &str {
        &self.salt
    }

    fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new(&self.bytes.into())
    }
}

/// The salt of the sealed value of `name`. A value written by a newer
/// cactus is [`Error::UnsupportedSecretFormat`]; a malformed one
/// [`Error::SecretCorrupt`].
pub fn salt_of<'a>(name: &str, sealed: &'a str) -> Result<&'a str> {
    Ok(split(name, sealed)?.0)
}

fn split<'a>(name: &str, sealed: &'a str) -> Result<(&'a str, &'a str)> {
    let Some(rest) = sealed.strip_prefix(SEALED_PREFIX) else {
        return Err(Error::UnsupportedSecretFormat(name.to_string()));
    };
    rest.split_once(':')
        .filter(|(salt, body)| !salt.is_empty() && !body.is_empty())
        .ok_or_else(|| Error::SecretCorrupt(format!("`{name}` isn't a valid encrypted value")))
}

fn value_aad(name: &str) -> Vec<u8> {
    format!("{VALUE_AAD_PREFIX}{name}").into_bytes()
}

/// `value` sealed under `name`, with a fresh nonce every time.
pub fn seal_value(key: &DataKey, name: &str, value: &str) -> Result<String> {
    let nonce: [u8; NONCE_LEN] = random()?;
    let aad = value_aad(name);
    let ciphertext = key
        .cipher()
        .encrypt(&XNonce::from(nonce), Payload { msg: value.as_bytes(), aad: &aad })
        .map_err(|_| Error::SecretCorrupt(format!("can't encrypt `{name}`")))?;
    let mut bytes = nonce.to_vec();
    bytes.extend_from_slice(&ciphertext);
    Ok(format!("{SEALED_PREFIX}{}:{}", key.salt, URL_SAFE_NO_PAD.encode(bytes)))
}

/// The plain value of `sealed`, stored under `name`. `None` when `key`
/// doesn't open it: a wrong password, a damaged value, or one moved from
/// another name.
pub fn open_value(key: &DataKey, name: &str, sealed: &str) -> Result<Option<String>> {
    let (salt, body) = split(name, sealed)?;
    if salt != key.salt {
        return Ok(None);
    }
    let corrupt = || Error::SecretCorrupt(format!("`{name}` isn't a valid encrypted value"));
    let bytes = URL_SAFE_NO_PAD.decode(body).map_err(|_| corrupt())?;
    if bytes.len() < NONCE_LEN {
        return Err(corrupt());
    }
    let (nonce, ciphertext) = bytes.split_at(NONCE_LEN);
    let nonce: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| corrupt())?;
    let aad = value_aad(name);
    let Ok(plain) = key.cipher().decrypt(&XNonce::from(nonce), Payload { msg: ciphertext, aad: &aad }) else {
        return Ok(None);
    };
    String::from_utf8(plain).map(Some).map_err(|_| corrupt())
}

fn random<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::is_sealed;

    fn password(p: &str) -> SecretString {
        p.to_string().into()
    }

    #[test]
    fn seal_and_open_round_trip() {
        let key = DataKey::generate(&password("password1")).unwrap();
        let sealed = seal_value(&key, "API_KEY", "s3cret ✓").unwrap();
        assert!(is_sealed(&sealed) && !sealed.contains("s3cret"));
        assert_eq!(salt_of("API_KEY", &sealed).unwrap(), key.salt());
        assert_eq!(open_value(&key, "API_KEY", &sealed).unwrap().unwrap(), "s3cret ✓");
        let empty = seal_value(&key, "E", "").unwrap();
        assert_eq!(open_value(&key, "E", &empty).unwrap().unwrap(), "");
    }

    #[test]
    fn the_same_password_and_salt_give_the_same_key() {
        let key = DataKey::generate(&password("password1")).unwrap();
        let sealed = seal_value(&key, "a", "v").unwrap();
        let again = DataKey::derive(&password("password1"), key.salt()).unwrap();
        assert_eq!(open_value(&again, "a", &sealed).unwrap().unwrap(), "v");
        let wrong = DataKey::derive(&password("password2"), key.salt()).unwrap();
        assert_eq!(open_value(&wrong, "a", &sealed).unwrap(), None);
    }

    #[test]
    fn tampering_or_another_name_fails() {
        let key = DataKey::generate(&password("password1")).unwrap();
        let sealed = seal_value(&key, "PROD_KEY", "v").unwrap();
        let (salt, body) = split("PROD_KEY", &sealed).unwrap();
        let mut bytes = URL_SAFE_NO_PAD.decode(body).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        let flipped = format!("{SEALED_PREFIX}{salt}:{}", URL_SAFE_NO_PAD.encode(bytes));
        assert_eq!(open_value(&key, "PROD_KEY", &flipped).unwrap(), None);
        assert_eq!(open_value(&key, "DEV_KEY", &sealed).unwrap(), None);
        assert!(matches!(open_value(&key, "a", "enc:v1:short"), Err(Error::SecretCorrupt(_))));
        assert!(matches!(open_value(&key, "a", "enc:v2:x:y"), Err(Error::UnsupportedSecretFormat(_))));
    }

    #[test]
    fn every_seal_uses_a_fresh_nonce() {
        let key = DataKey::generate(&password("password1")).unwrap();
        assert_ne!(seal_value(&key, "a", "v").unwrap(), seal_value(&key, "a", "v").unwrap());
    }

    #[test]
    fn passwords_need_eight_characters() {
        assert!(matches!(password("seven77").check_length(), Err(Error::PasswordTooShort)));
        assert!(password("ééééééé✓").check_length().is_ok());
    }
}
