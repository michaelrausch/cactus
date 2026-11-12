//! Reading and writing secrets files: the only code that touches them, so
//! nothing can bypass encryption by accident.

#[cfg(feature = "encryption")]
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

#[cfg(feature = "encryption")]
use super::cipher::{DataKey, open_value, salt_of, seal_value};
use super::is_sealed;
#[cfg(feature = "encryption")]
use super::keyring;
use crate::critter::validate_critter_id;
use crate::env::{Env, ensure_critter_workspace};
#[cfg(feature = "encryption")]
use crate::state;
use crate::storage::{read_json, write_json};
use crate::{Error, Result, layout};

/// The file name, in `.cactus/` and in each critter's folder there.
pub const SECRETS_FILE: &str = "secrets.json";

/// Every secrets file a project can have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretFile {
    /// `.cactus/secrets.json`.
    Project,
    /// `.cactus/critters/<id>/secrets.json`.
    Critter(String),
}

impl SecretFile {
    fn path(&self, project: &Path) -> Result<PathBuf> {
        Ok(match self {
            Self::Project => layout::workspace_dir(project).join(SECRETS_FILE),
            Self::Critter(id) => {
                validate_critter_id(id)?;
                layout::critter_workspace_dir(project, id).join(SECRETS_FILE)
            }
        })
    }

    /// The project file plus every critter secrets file on disk. Scanning
    /// the workspace finds files of critters whose YAML is gone, too.
    pub fn all(project: &Path) -> Vec<Self> {
        let mut files = vec![Self::Project];
        let dir = layout::workspace_dir(project).join(layout::CRITTERS_DIR);
        let mut ids: Vec<String> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter(|id| validate_critter_id(id).is_ok())
            .filter(|id| layout::critter_workspace_dir(project, id).join(SECRETS_FILE).is_file())
            .collect();
        ids.sort();
        files.extend(ids.into_iter().map(Self::Critter));
        files
    }
}

/// Held by every write and by enable, disable, reset and unlock, so a save
/// can't race a migration.
static SECRETS_WRITE: Mutex<()> = Mutex::new(());

pub(super) fn write_lock() -> MutexGuard<'static, ()> {
    SECRETS_WRITE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The file as stored: sealed values stay sealed.
pub(super) fn read_raw(project: &Path, file: &SecretFile) -> Result<Env> {
    read_json(&file.path(project)?)
}

pub(super) fn write_raw(project: &Path, file: &SecretFile, env: &Env) -> Result<()> {
    match file {
        SecretFile::Project => layout::ensure_workspace(project).map(drop)?,
        SecretFile::Critter(id) => ensure_critter_workspace(project, id)?,
    }
    #[cfg(all(test, feature = "encryption"))]
    fail_point::before_write()?;
    write_json(&file.path(project)?, env)
}

/// Every secrets file, as stored.
pub(super) fn read_all_raw(project: &Path) -> Result<Vec<(SecretFile, Env)>> {
    SecretFile::all(project).into_iter().map(|file| Ok((file.clone(), read_raw(project, &file)?))).collect()
}

/// One sealed value per salt in `files`, as `salt → (name, value)`.
#[cfg(feature = "encryption")]
pub(super) fn sealed_by_salt(files: &[(SecretFile, Env)]) -> Result<BTreeMap<String, (String, String)>> {
    let mut samples = BTreeMap::new();
    for (name, value) in files.iter().flat_map(|(_, env)| env).filter(|(_, value)| is_sealed(value)) {
        let salt = salt_of(name, value)?;
        samples.entry(salt.to_string()).or_insert_with(|| (name.clone(), value.clone()));
    }
    Ok(samples)
}

/// Whether encryption is on: switched on in `.cactus/state.json`, or any
/// value is sealed (so a lost state file never exposes new values).
#[cfg(feature = "encryption")]
pub(super) fn is_on(project: &Path, files: &[(SecretFile, Env)]) -> bool {
    state::load(project).secrets.is_some() || files.iter().flat_map(|(_, env)| env.values()).any(|v| is_sealed(v))
}

/// `value` of `name` opened with whichever held key matches its salt.
/// No key for the salt: [`Error::SecretsLocked`].
#[cfg(feature = "encryption")]
pub(super) fn open_held(project: &Path, name: &str, value: &str) -> Result<String> {
    if !is_sealed(value) {
        return Ok(value.to_string());
    }
    let key = keyring::for_salt(project, salt_of(name, value)?).ok_or(Error::SecretsLocked)?;
    open_value(&key, name, value)?.ok_or_else(|| Error::SecretCorrupt(format!("`{name}` doesn't decrypt")))
}

/// Without the `encryption` feature a sealed value can never be opened, and
/// is never used as a literal either.
#[cfg(not(feature = "encryption"))]
pub(super) fn open_held(_project: &Path, _name: &str, value: &str) -> Result<String> {
    if is_sealed(value) {
        return Err(Error::SecretsLocked);
    }
    Ok(value.to_string())
}

/// Every value of `env` opened with the held keys.
pub(super) fn open_all_held(project: &Path, env: &Env) -> Result<Env> {
    env.iter().map(|(name, value)| Ok((name.clone(), open_held(project, name, value)?))).collect()
}

/// Every value sealed under its name with `key`, with a fresh nonce.
#[cfg(feature = "encryption")]
pub(super) fn seal_all(key: &DataKey, env: &Env) -> Result<Env> {
    env.iter().map(|(name, value)| Ok((name.clone(), seal_value(key, name, value)?))).collect()
}

/// The file's values, opened. Plain values are used as they are; a sealed
/// value needs its key: while locked that is [`Error::SecretsLocked`], so
/// a sealed value is never used as a literal. A file with only plain
/// values never needs the password.
pub fn read(project: &Path, file: &SecretFile) -> Result<Env> {
    open_all_held(project, &read_raw(project, file)?)
}

/// The file's variable names. Never needs the key, so it works while locked.
pub fn names(project: &Path, file: &SecretFile) -> Result<Vec<String>> {
    Ok(read_raw(project, file)?.into_keys().collect())
}

/// Replace the file's values with `env`. Encryption off: stored as plain
/// text, as before. On and unlocked: every value sealed. On and locked:
/// [`Error::SecretsLocked`], since new values can't be sealed.
pub fn write(project: &Path, file: &SecretFile, env: &Env) -> Result<()> {
    let _guard = write_lock();
    #[cfg(feature = "encryption")]
    if is_on(project, &read_all_raw(project)?) {
        let key = keyring::current(project).ok_or(Error::SecretsLocked)?;
        return write_raw(project, file, &seal_all(&key, env)?);
    }
    write_raw(project, file, env)
}

/// Whether resolving `files` would fail with [`Error::SecretsLocked`]: one
/// holds a sealed value whose key isn't held.
pub fn locked_for(project: &Path, files: &[SecretFile]) -> bool {
    files
        .iter()
        .filter_map(|file| read_raw(project, file).ok())
        .any(|env| env.iter().any(|(name, value)| is_sealed(value) && !key_held(project, name, value)))
}

#[cfg(feature = "encryption")]
fn key_held(project: &Path, name: &str, sealed: &str) -> bool {
    salt_of(name, sealed).is_ok_and(|salt| keyring::for_salt(project, salt).is_some())
}

#[cfg(not(feature = "encryption"))]
fn key_held(_project: &Path, _name: &str, _sealed: &str) -> bool {
    false
}

/// Make the nth secrets-file write from now (on this thread) fail, to check
/// every intermediate state of enable, disable and reset is readable.
#[cfg(all(test, feature = "encryption"))]
pub(super) mod fail_point {
    use std::cell::Cell;

    use crate::{Error, Result};

    thread_local! {
        static WRITES_LEFT: Cell<Option<usize>> = const { Cell::new(None) };
    }

    pub fn fail_after(writes: Option<usize>) {
        WRITES_LEFT.with(|left| left.set(writes));
    }

    pub fn before_write() -> Result<()> {
        WRITES_LEFT.with(|left| match left.get() {
            Some(0) => Err(Error::Io(std::io::Error::other("injected failure"))),
            Some(n) => {
                left.set(Some(n - 1));
                Ok(())
            }
            None => Ok(()),
        })
    }
}
