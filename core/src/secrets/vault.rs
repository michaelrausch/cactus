//! Secrets and context files at rest. Each is sealed whole:
//!
//! ```text
//! "CVLT" ‖ 0x01 ‖ nonce(24) ‖ XChaCha20-Poly1305(json)
//! ```
//!
//! With no password the key is built in, so the files aren't plain text but
//! anyone with cactus can open them. A password replaces it with
//! Argon2id(password, salt); `.cactus/vault` (`salt(16) ‖ sealed(check ‖
//! totp secret?)`) marks that, and until [`unlock`] reads of those files fail
//! as [`Error::Locked`]. With a TOTP secret, unlocking also takes a code (see
//! [`super::totp`]). A file still in plain JSON is read as-is and sealed on
//! its next write.

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::SECRETS_FILE;
use super::totp::{self, TotpSetup};
use crate::storage::write_atomic;
use crate::{Error, Result, layout, now_unix_millis};

const MAGIC: &[u8] = b"CVLT";
const VERSION: u8 = 1;
const NONCE_LEN: usize = 24;
const SALT_LEN: usize = 16;
const AAD: &[u8] = b"cactus/vault/v1";
const CHECK: &[u8] = b"cactus";
const VAULT_FILE: &str = "vault";
const BUILT_IN_KEY: Key = *b"cactus/vault/v1/built-in-key-32b";

/// Argon2id cost: 64 MiB and 3 passes. Tests use the minimum.
#[cfg(not(test))]
const COST: (u32, u32, u32) = (64 * 1024, 3, 1);
#[cfg(test)]
const COST: (u32, u32, u32) = (8, 1, 1);

type Key = [u8; 32];

/// Unlocked projects' keys, by canonical project folder.
static KEYS: Mutex<BTreeMap<PathBuf, Key>> = Mutex::new(BTreeMap::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum VaultStatus {
    NoPassword,
    Locked,
    Unlocked,
}

pub fn status(project: &Path) -> VaultStatus {
    if !vault_file(project).exists() {
        VaultStatus::NoPassword
    } else if cached_key(project).is_some() {
        VaultStatus::Unlocked
    } else {
        VaultStatus::Locked
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Unlocked {
    Unlocked,
    /// The password is right; ask for the authenticator code and try again with it.
    CodeRequired,
}

/// Check `password` (and `code`, with TOTP on) and keep the key for this
/// process. Nothing to do without a password set.
pub fn unlock(project: &Path, password: &str, code: Option<&str>) -> Result<Unlocked> {
    let Some(opened) = verify(project, password)? else {
        return Ok(Unlocked::Unlocked);
    };
    if let Some(secret) = &opened.totp {
        match code {
            None => return Ok(Unlocked::CodeRequired),
            Some(code) if !totp::verify(secret, code, now_secs()) => return Err(Error::WrongCode),
            Some(_) => {}
        }
    }
    keys().insert(root(project), opened.key);
    Ok(Unlocked::Unlocked)
}

/// Whether unlocking takes a code. False while locked or with no password.
pub fn totp_enabled(project: &Path) -> bool {
    opened_with_cached_key(project).is_ok_and(|opened| opened.totp.is_some())
}

/// A fresh secret to add to an authenticator app; [`enable_totp`] turns it on.
pub fn start_totp(project: &Path) -> Result<TotpSetup> {
    opened_with_cached_key(project)?;
    let secret: [u8; totp::SECRET_LEN] = random()?;
    let account = project.file_name().map_or_else(|| "project".into(), |name| name.to_string_lossy());
    totp::setup(&secret, &account)
}

/// Require a code to unlock, once `code` shows the app has `secret`.
pub fn enable_totp(project: &Path, secret: &str, code: &str) -> Result<()> {
    let opened = opened_with_cached_key(project)?;
    let secret = totp::decode(secret)?;
    if !totp::verify(&secret, code, now_secs()) {
        return Err(Error::WrongCode);
    }
    write_vault(project, &opened.salt, &opened.key, Some(&secret))
}

/// Stop asking for a code; takes a current one.
pub fn disable_totp(project: &Path, code: &str) -> Result<()> {
    let opened = opened_with_cached_key(project)?;
    let Some(secret) = &opened.totp else { return Ok(()) };
    if !totp::verify(secret, code, now_secs()) {
        return Err(Error::WrongCode);
    }
    write_vault(project, &opened.salt, &opened.key, None)
}

/// For a forgotten password: delete every secrets and context file and the
/// password, leaving the project unlocked with nothing in them.
pub fn wipe(project: &Path) -> Result<()> {
    let _guard = crate::env::write_lock();
    let vault = vault_file(project);
    for path in sealed_files(project)?.into_iter().chain([vault].into_iter().filter(|f| f.exists())) {
        fs::remove_file(&path).map_err(Error::file(&path))?;
    }
    keys().remove(&root(project));
    Ok(())
}

/// Set, change (`new`) or remove (`None`) the password, re-sealing every
/// file. `current` is required while a password is set.
pub fn set_password(project: &Path, current: Option<&str>, new: Option<&str>) -> Result<()> {
    if new.is_some_and(str::is_empty) {
        return Err(Error::EmptyPassword);
    }
    let _guard = crate::env::write_lock();
    let (old, totp) = match verify(project, current.unwrap_or_default())? {
        Some(opened) => (opened.key, opened.totp),
        None if vault_file(project).exists() => return Err(Error::WrongPassword),
        None => (BUILT_IN_KEY, None),
    };
    let mut plain = Vec::new();
    for path in sealed_files(project)? {
        let bytes = fs::read(&path).map_err(Error::file(&path))?;
        let json =
            if is_plain(&bytes) { bytes } else { open(&old, &bytes).ok_or(Error::Undecryptable(path.clone()))? };
        plain.push((path, json));
    }
    let (key, salt) = match new {
        Some(password) => {
            let salt: [u8; SALT_LEN] = random()?;
            (derive(password, &salt)?, Some(salt))
        }
        None => (BUILT_IN_KEY, None),
    };
    for (path, json) in plain {
        write_atomic(&path, &seal(&key, &json)?)?;
    }
    let file = vault_file(project);
    if let Some(salt) = salt {
        // A new password keeps the TOTP secret; removing it drops both.
        write_vault(project, &salt, &key, totp.as_deref())?;
        keys().insert(root(project), key);
    } else {
        if file.exists() {
            fs::remove_file(&file).map_err(Error::file(&file))?;
        }
        keys().remove(&root(project));
    }
    Ok(())
}

/// Read a sealed (or plain) JSON file; a missing one is `T::default()`.
pub(crate) fn read_json<T: DeserializeOwned + Default>(project: &Path, path: &Path) -> Result<T> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(T::default()),
        Err(e) => return Err(Error::file(path)(e)),
    };
    if is_plain(&bytes) {
        return Ok(serde_json::from_slice(&bytes)?);
    }
    let json = open(&key(project)?, &bytes).ok_or_else(|| Error::Undecryptable(path.to_path_buf()))?;
    Ok(serde_json::from_slice(&json)?)
}

/// Seal `value` and write it atomically.
pub(crate) fn write_json<T: Serialize>(project: &Path, path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_vec(value)?;
    write_atomic(path, &seal(&key(project)?, &json)?)
}

fn vault_file(project: &Path) -> PathBuf {
    layout::workspace_dir(project).join(VAULT_FILE)
}

/// Every sealed file the project has: secrets, context, cookies and OAuth2 tokens.
fn sealed_files(project: &Path) -> Result<Vec<PathBuf>> {
    let workspace = layout::workspace_dir(project);
    let mut files = vec![workspace.join(SECRETS_FILE), layout::captured_file(project), layout::tokens_file(project)];
    let critters = workspace.join(layout::CRITTERS_DIR);
    match fs::read_dir(&critters) {
        Ok(entries) => {
            for entry in entries {
                let dir = entry.map_err(Error::file(&critters))?.path();
                let id = dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
                files.push(dir.join(SECRETS_FILE));
                files.push(layout::critter_context_file(project, &id));
                files.push(layout::critter_cookies_file(project, &id));
                files.push(layout::critter_tokens_file(project, &id));
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(Error::file(&critters)(e)),
    }
    files.retain(|f| f.is_file());
    Ok(files)
}

fn root(project: &Path) -> PathBuf {
    fs::canonicalize(project).unwrap_or_else(|_| project.to_path_buf())
}

fn keys() -> std::sync::MutexGuard<'static, BTreeMap<PathBuf, Key>> {
    KEYS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn cached_key(project: &Path) -> Option<Key> {
    keys().get(&root(project)).copied()
}

fn key(project: &Path) -> Result<Key> {
    if !vault_file(project).exists() {
        return Ok(BUILT_IN_KEY);
    }
    cached_key(project).ok_or(Error::Locked)
}

/// The vault file opened: its key and salt, and the TOTP secret if it has one.
struct Opened {
    key: Key,
    salt: Vec<u8>,
    totp: Option<Vec<u8>>,
}

/// The vault file's salt and sealed part; `None` with no password set.
fn read_vault(project: &Path) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
    let file = vault_file(project);
    let mut vault = match fs::read(&file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::file(&file)(e)),
    };
    if vault.len() < SALT_LEN {
        return Err(Error::Undecryptable(file));
    }
    let sealed = vault.split_off(SALT_LEN);
    Ok(Some((vault, sealed)))
}

/// What `key` opens the sealed part to, if it's the right key.
fn open_vault(key: Key, salt: Vec<u8>, sealed: &[u8]) -> Option<Opened> {
    let plain = open(&key, sealed)?;
    let rest = plain.strip_prefix(CHECK)?;
    Some(Opened { key, salt, totp: (!rest.is_empty()).then(|| rest.to_vec()) })
}

/// The vault as `password` opens it; `None` with no vault.
fn verify(project: &Path, password: &str) -> Result<Option<Opened>> {
    let Some((salt, sealed)) = read_vault(project)? else { return Ok(None) };
    let key = derive(password, &salt)?;
    open_vault(key, salt, &sealed).map(Some).ok_or(Error::WrongPassword)
}

/// The vault as the key from [`unlock`] opens it.
fn opened_with_cached_key(project: &Path) -> Result<Opened> {
    let Some((salt, sealed)) = read_vault(project)? else { return Err(Error::NoPassword) };
    let key = cached_key(project).ok_or(Error::Locked)?;
    open_vault(key, salt, &sealed).ok_or_else(|| Error::Undecryptable(vault_file(project)))
}

fn write_vault(project: &Path, salt: &[u8], key: &Key, totp: Option<&[u8]>) -> Result<()> {
    let mut plain = CHECK.to_vec();
    plain.extend_from_slice(totp.unwrap_or_default());
    let mut vault = salt.to_vec();
    vault.extend(seal(key, &plain)?);
    layout::ensure_workspace(project)?;
    write_atomic(&vault_file(project), &vault)
}

fn now_secs() -> u64 {
    now_unix_millis() / 1000
}

fn derive(password: &str, salt: &[u8]) -> Result<Key> {
    let (memory_kib, iterations, parallelism) = COST;
    let failed = |e: argon2::Error| Error::Tool(format!("key derivation: {e}"));
    let params = Params::new(memory_kib, iterations, parallelism, Some(32)).map_err(failed)?;
    let mut key = [0; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(failed)?;
    Ok(key)
}

/// Plain JSON from before files were sealed.
fn is_plain(bytes: &[u8]) -> bool {
    !bytes.starts_with(MAGIC)
}

fn seal(key: &Key, plain: &[u8]) -> Result<Vec<u8>> {
    let nonce: [u8; NONCE_LEN] = random()?;
    let sealed = XChaCha20Poly1305::new(&(*key).into())
        .encrypt(&XNonce::from(nonce), Payload { msg: plain, aad: AAD })
        .map_err(|_| Error::Tool("can't encrypt".into()))?;
    let mut out = MAGIC.to_vec();
    out.push(VERSION);
    out.extend_from_slice(&nonce);
    out.extend(sealed);
    Ok(out)
}

fn open(key: &Key, bytes: &[u8]) -> Option<Vec<u8>> {
    let rest = bytes.strip_prefix(MAGIC)?.strip_prefix(&[VERSION])?;
    if rest.len() < NONCE_LEN {
        return None;
    }
    let (nonce, sealed) = rest.split_at(NONCE_LEN);
    let nonce: [u8; NONCE_LEN] = nonce.try_into().ok()?;
    XChaCha20Poly1305::new(&(*key).into()).decrypt(&XNonce::from(nonce), Payload { msg: sealed, aad: AAD }).ok()
}

fn random<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{self, Env, EnvFile};

    fn values(pairs: &[(&str, &str)]) -> Env {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    fn forget(project: &Path) {
        keys().remove(&root(project));
    }

    #[test]
    fn files_are_sealed_without_a_password() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        env::save(p, &EnvFile::Secrets, &values(&[("TOKEN", "s3cret")])).unwrap();
        let raw = fs::read(layout::workspace_dir(p).join(SECRETS_FILE)).unwrap();
        assert!(raw.starts_with(MAGIC) && !raw.windows(6).any(|w| w == b"s3cret"));
        assert_eq!(env::load(p, &EnvFile::Secrets).unwrap(), values(&[("TOKEN", "s3cret")]));
        assert_eq!(status(p), VaultStatus::NoPassword);
    }

    #[test]
    fn plain_json_still_reads_and_is_sealed_on_write() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        layout::ensure_workspace(p).unwrap();
        fs::write(layout::captured_file(p), r#"{"a": "1"}"#).unwrap();
        assert_eq!(env::load(p, &EnvFile::Captured).unwrap(), values(&[("a", "1")]));
        env::merge_captured(p, &values(&[("b", "2")])).unwrap();
        assert!(fs::read(layout::captured_file(p)).unwrap().starts_with(MAGIC));
        assert_eq!(env::load(p, &EnvFile::Captured).unwrap(), values(&[("a", "1"), ("b", "2")]));
    }

    #[test]
    fn a_password_locks_until_unlocked() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        env::save(p, &EnvFile::Secrets, &values(&[("A", "1")])).unwrap();
        env::save(p, &EnvFile::CritterSecrets("bob".into()), &values(&[("B", "2")])).unwrap();
        env::merge_context(p, "bob", &values(&[("C", "3")]), &crate::ProjectPath::parse("r.yml").unwrap()).unwrap();
        set_password(p, None, Some("hunter2")).unwrap();
        assert_eq!(status(p), VaultStatus::Unlocked);

        forget(p);
        assert_eq!(status(p), VaultStatus::Locked);
        assert!(matches!(env::load(p, &EnvFile::Secrets), Err(Error::Locked)));
        assert!(matches!(unlock(p, "nope", None), Err(Error::WrongPassword)));
        assert_eq!(unlock(p, "hunter2", None).unwrap(), Unlocked::Unlocked);
        assert_eq!(env::load(p, &EnvFile::Secrets).unwrap(), values(&[("A", "1")]));
        assert_eq!(env::load(p, &EnvFile::CritterSecrets("bob".into())).unwrap(), values(&[("B", "2")]));
        assert_eq!(env::load(p, &EnvFile::CritterContext("bob".into())).unwrap(), values(&[("C", "3")]));
    }

    #[test]
    fn changing_and_removing_the_password_reseals_everything() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        env::save(p, &EnvFile::Secrets, &values(&[("A", "1")])).unwrap();
        env::save(p, &EnvFile::CritterSecrets("bob".into()), &values(&[("B", "2")])).unwrap();
        set_password(p, None, Some("one")).unwrap();
        assert!(matches!(set_password(p, Some("wrong"), Some("two")), Err(Error::WrongPassword)));
        assert!(matches!(set_password(p, None, None), Err(Error::WrongPassword)));
        assert!(matches!(set_password(p, Some("one"), Some("")), Err(Error::EmptyPassword)));
        set_password(p, Some("one"), Some("two")).unwrap();

        forget(p);
        assert!(matches!(unlock(p, "one", None), Err(Error::WrongPassword)));
        unlock(p, "two", None).unwrap();
        assert_eq!(env::load(p, &EnvFile::CritterSecrets("bob".into())).unwrap(), values(&[("B", "2")]));

        set_password(p, Some("two"), None).unwrap();
        forget(p);
        assert_eq!(status(p), VaultStatus::NoPassword);
        assert_eq!(env::load(p, &EnvFile::Secrets).unwrap(), values(&[("A", "1")]));
    }

    #[test]
    fn totp_adds_a_code_to_unlocking_and_survives_a_password_change() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        assert!(matches!(start_totp(p), Err(Error::NoPassword)));
        set_password(p, None, Some("pw")).unwrap();
        let setup = start_totp(p).unwrap();
        let secret = totp::decode(&setup.secret).unwrap();
        let code = || totp::code_at(&secret, now_secs());
        assert!(matches!(enable_totp(p, &setup.secret, "000000x"), Err(Error::WrongCode)));
        enable_totp(p, &setup.secret, &code()).unwrap();
        assert!(totp_enabled(p));

        forget(p);
        assert_eq!(unlock(p, "pw", None).unwrap(), Unlocked::CodeRequired);
        assert_eq!(status(p), VaultStatus::Locked);
        assert!(matches!(unlock(p, "pw", Some("000000x")), Err(Error::WrongCode)));
        assert_eq!(unlock(p, "pw", Some(&code())).unwrap(), Unlocked::Unlocked);

        set_password(p, Some("pw"), Some("pw2")).unwrap();
        forget(p);
        assert_eq!(unlock(p, "pw2", None).unwrap(), Unlocked::CodeRequired);
        unlock(p, "pw2", Some(&code())).unwrap();

        assert!(matches!(disable_totp(p, "000000x"), Err(Error::WrongCode)));
        disable_totp(p, &code()).unwrap();
        forget(p);
        assert_eq!(unlock(p, "pw2", None).unwrap(), Unlocked::Unlocked);
    }

    #[test]
    fn wiping_clears_a_forgotten_password_and_everything_under_it() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        env::save(p, &EnvFile::CritterSecrets("bob".into()), &values(&[("B", "2")])).unwrap();
        set_password(p, None, Some("lost")).unwrap();
        forget(p);
        wipe(p).unwrap();
        assert_eq!(status(p), VaultStatus::NoPassword);
        assert!(env::load(p, &EnvFile::CritterSecrets("bob".into())).unwrap().is_empty());
    }
}
