//! Pro license keys, checked offline.
//!
//! A key is `CACTUS-PRO-` and the base32 of the licensee as JSON followed by
//! an Ed25519 signature over that JSON. Only the public half of the signing
//! key is in the app; `license-gen` issues keys with the private half.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{PoisonError, RwLock};

use data_encoding::BASE32_NOPAD;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::storage::write_atomic;
use crate::{Error, Result};

/// The public half of the signing key that issues keys.
const PUBLIC_KEY: [u8; 32] = [
    0xe5, 0x3d, 0x64, 0x93, 0x63, 0x8a, 0x06, 0xf6, 0x71, 0xa0, 0x8e, 0xc0, 0xd3, 0x10, 0xa1, 0xa1, 0x0a, 0x3a, 0x6c,
    0xc8, 0x09, 0x05, 0x4a, 0xd3, 0xfb, 0xed, 0xc5, 0x7c, 0x8a, 0x21, 0x82, 0x69,
];

const PREFIX: &str = "CACTUS-PRO-";

/// The key's file in the app's config folder.
pub const FILE_NAME: &str = "license.key";

/// Who a key was issued to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct License {
    /// Tells keys apart, e.g. to find one that was shared.
    pub id: String,
    pub name: String,
    pub email: String,
}

/// The license this process was activated with.
static ACTIVE: RwLock<Option<License>> = RwLock::new(None);

/// The active license, if any.
pub fn active() -> Option<License> {
    ACTIVE.read().unwrap_or_else(PoisonError::into_inner).clone()
}

/// Check `key` and read who it was issued to.
pub fn parse(key: &str) -> Result<License> {
    parse_with(key, &PUBLIC_KEY)
}

fn parse_with(key: &str, public_key: &[u8; 32]) -> Result<License> {
    let compact: String = key.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase();
    let encoded = compact.strip_prefix(PREFIX).ok_or(Error::InvalidLicense)?;
    let bytes = BASE32_NOPAD.decode(encoded.as_bytes()).map_err(|_| Error::InvalidLicense)?;
    let split = bytes.len().checked_sub(Signature::BYTE_SIZE).ok_or(Error::InvalidLicense)?;
    let (payload, signature) = bytes.split_at(split);
    let signature = Signature::from_slice(signature).map_err(|_| Error::InvalidLicense)?;
    let public_key = VerifyingKey::from_bytes(public_key).map_err(|_| Error::InvalidLicense)?;
    public_key.verify_strict(payload, &signature).map_err(|_| Error::InvalidLicense)?;
    serde_json::from_slice(payload).map_err(|_| Error::InvalidLicense)
}

/// A key for `license`, signed with the private half of the signing key.
pub fn issue(license: &License, signing_key: &[u8; 32]) -> Result<String> {
    let mut bytes = serde_json::to_vec(license)?;
    let signature = SigningKey::from_bytes(signing_key).sign(&bytes);
    bytes.extend_from_slice(&signature.to_bytes());
    Ok(format!("{PREFIX}{}", BASE32_NOPAD.encode(&bytes)))
}

/// The public half of `signing_key`, for [`PUBLIC_KEY`].
pub fn public_key(signing_key: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(signing_key).verifying_key().to_bytes()
}

/// Activate with the key saved in `dir`, if there is a valid one.
pub fn load(dir: &Path) -> Option<License> {
    load_with(dir, &PUBLIC_KEY)
}

/// Check `key`, save it in `dir` and activate with it.
pub fn activate(dir: &Path, key: &str) -> Result<License> {
    activate_with(dir, key, &PUBLIC_KEY)
}

fn load_with(dir: &Path, public_key: &[u8; 32]) -> Option<License> {
    let path = dir.join(FILE_NAME);
    let key = match fs::read_to_string(&path) {
        Ok(key) => key,
        Err(error) if error.kind() == ErrorKind::NotFound => return None,
        Err(error) => {
            crate::best_effort::log("read the license key", Err::<(), _>(error));
            return None;
        }
    };
    let license = parse_with(&key, public_key).ok()?;
    set_active(Some(license.clone()));
    Some(license)
}

fn activate_with(dir: &Path, key: &str, public_key: &[u8; 32]) -> Result<License> {
    let license = parse_with(key, public_key)?;
    fs::create_dir_all(dir).map_err(Error::file(dir))?;
    write_atomic(&dir.join(FILE_NAME), key.trim().as_bytes())?;
    set_active(Some(license.clone()));
    Ok(license)
}

fn set_active(license: Option<License>) {
    *ACTIVE.write().unwrap_or_else(PoisonError::into_inner) = license;
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIGNING_KEY: [u8; 32] = [7; 32];

    fn licensee() -> License {
        License { id: "a1b2c3d4".into(), name: "Ada".into(), email: "ada@example.com".into() }
    }

    #[test]
    fn an_issued_key_reads_back() {
        let key = issue(&licensee(), &SIGNING_KEY).unwrap();
        assert_eq!(parse_with(&key, &public_key(&SIGNING_KEY)).unwrap(), licensee());
    }

    /// The website issues keys too (`marketing_site/worker/license.ts`); its
    /// test expects this same key, so the two can't drift apart.
    #[test]
    fn keys_match_the_websites() {
        let license = License { id: "test".into(), name: "Ada".into(), email: "ada@example.com".into() };
        assert_eq!(
            issue(&license, &SIGNING_KEY).unwrap(),
            "CACTUS-PRO-PMRGSZBCHIRHIZLTOQRCYITOMFWWKIR2EJAWIYJCFQRGK3LBNFWCEORCMFSGCQDFPBQW24DMMUXGG33NEJ6UBVGZWFY4WS6L6HO6BCH3226F5RRE53IKVNVXIXDK7QC4ZSIY5UDQELRLYYTOYGTDX3F55Z76UKGDZKVEORVRJUHQ4BRCDT4TIIN2A4"
        );
    }

    #[test]
    fn pasted_keys_may_be_wrapped_or_lowercase() {
        let key = issue(&licensee(), &SIGNING_KEY).unwrap();
        let (head, tail) = key.split_at(40);
        let pasted = format!("  {}\n{}  ", head.to_lowercase(), tail);
        assert_eq!(parse_with(&pasted, &public_key(&SIGNING_KEY)).unwrap(), licensee());
    }

    #[test]
    fn another_signing_key_is_refused() {
        let key = issue(&licensee(), &[8; 32]).unwrap();
        assert!(matches!(parse_with(&key, &public_key(&SIGNING_KEY)), Err(Error::InvalidLicense)));
    }

    #[test]
    fn a_changed_licensee_is_refused() {
        let key = issue(&licensee(), &SIGNING_KEY).unwrap();
        let other = issue(&License { name: "Eve".into(), ..licensee() }, &SIGNING_KEY).unwrap();
        // Eve's payload with Ada's signature.
        let decode = |key: &str| BASE32_NOPAD.decode(&key.as_bytes()[PREFIX.len()..]).unwrap();
        let (ada, eve) = (decode(&key), decode(&other));
        let mut forged = eve[..eve.len() - Signature::BYTE_SIZE].to_vec();
        forged.extend_from_slice(&ada[ada.len() - Signature::BYTE_SIZE..]);
        let forged = format!("{PREFIX}{}", BASE32_NOPAD.encode(&forged));
        assert!(matches!(parse_with(&forged, &public_key(&SIGNING_KEY)), Err(Error::InvalidLicense)));
    }

    #[test]
    fn garbage_is_refused() {
        let public = public_key(&SIGNING_KEY);
        for key in ["", "CACTUS-PRO-", "CACTUS-PRO-!!!", "CACTUS-PRO-AAAA", "hello"] {
            assert!(matches!(parse_with(key, &public), Err(Error::InvalidLicense)), "{key:?}");
        }
    }

    #[test]
    fn activating_saves_the_key_for_next_time() {
        let dir = tempfile::tempdir().unwrap();
        let public = public_key(&SIGNING_KEY);
        assert_eq!(load_with(dir.path(), &public), None);
        assert!(activate_with(dir.path(), "CACTUS-PRO-AAAA", &public).is_err());
        assert!(!dir.path().join(FILE_NAME).exists(), "a bad key isn't saved");

        let key = issue(&licensee(), &SIGNING_KEY).unwrap();
        activate_with(dir.path(), &format!("{key}\n"), &public).unwrap();
        assert_eq!(active(), Some(licensee()));
        assert_eq!(load_with(dir.path(), &public), Some(licensee()));
    }
}
