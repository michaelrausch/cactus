//! Turning encryption on and off, unlocking, and changing the password.
//!
//! Plain values are valid while encryption is on, and every sealed value
//! carries its own salt, so every intermediate state is a valid state: a
//! crash at any point leaves every secret readable, and at worst some
//! values plain (or under the previous password) until the next unlock.

use std::path::Path;

use super::cipher::{DataKey, SecretString, open_value};
use super::store::{SecretFile, is_on, open_all_held, read_all_raw, seal_all, sealed_by_salt, write_lock, write_raw};
use super::{SecretsInfo, SecretsStatus, UnlockReport, is_sealed, keyring};
use crate::env::Env;
use crate::state::{self, DEFAULT_AUTO_LOCK_MINUTES, SecretSettings};
use crate::{Error, Result};

pub fn info(project: &Path) -> Result<SecretsInfo> {
    let files = read_all_raw(project)?;
    let samples = sealed_by_salt(&files)?;
    let status = if !is_on(project, &files) {
        SecretsStatus::Off
    } else if keyring::current(project).is_some()
        && samples.keys().all(|salt| keyring::for_salt(project, salt).is_some())
    {
        SecretsStatus::Unlocked
    } else {
        SecretsStatus::Locked
    };
    let auto_lock_minutes = match state::load(project).secrets {
        Some(settings) => settings.auto_lock_minutes,
        None => Some(DEFAULT_AUTO_LOCK_MINUTES),
    };
    Ok(SecretsInfo { status, auto_lock_minutes, has_sealed_values: !samples.is_empty() })
}

/// Whether encryption is on, and if so whether this process holds the keys.
pub fn status(project: &Path) -> Result<SecretsStatus> {
    Ok(info(project)?.status)
}

/// For each salt in use, the key `password` gives, if it opens that salt's
/// values: `(opened, salts it didn't open)`.
fn derive_for(password: &SecretString, files: &[(SecretFile, Env)]) -> Result<(Vec<DataKey>, usize)> {
    let mut opened = Vec::new();
    let mut failed = 0;
    for (salt, (name, value)) in sealed_by_salt(files)? {
        let key = DataKey::derive(password, &salt)?;
        if open_value(&key, &name, &value)?.is_some() {
            opened.push(key);
        } else {
            failed += 1;
        }
    }
    Ok((opened, failed))
}

/// The keys for every salt in use, all opened by `password`; otherwise
/// [`Error::WrongPassword`].
fn verify(password: &SecretString, files: &[(SecretFile, Env)]) -> Result<Vec<DataKey>> {
    match derive_for(password, files)? {
        (keys, 0) => Ok(keys),
        _ => Err(Error::WrongPassword),
    }
}

fn set_settings(project: &Path, settings: Option<SecretSettings>) -> Result<()> {
    state::update(project, |s| s.secrets = settings)
}

/// Turn encryption on with `password`, leaving the project unlocked, and
/// seal every value of every secrets file.
pub fn enable(project: &Path, password: &SecretString, auto_lock_minutes: Option<u32>) -> Result<()> {
    password.check_length()?;
    let _guard = write_lock();
    let files = read_all_raw(project)?;
    if is_on(project, &files) {
        return Err(Error::EncryptionAlreadyOn);
    }
    set_settings(project, Some(SecretSettings { auto_lock_minutes }))?;
    let key = keyring::insert(project, DataKey::generate(password)?);
    for (file, env) in &files {
        if env.is_empty() {
            continue;
        }
        write_raw(project, file, &seal_all(&key, env)?)?;
        if &super::read(project, file)? != env {
            return Err(Error::SecretCorrupt("a sealed file didn't read back as written".into()));
        }
    }
    Ok(())
}

/// Turn encryption off: every value is written back as plain text. Needs
/// the password even when unlocked. A wrong password changes nothing.
pub fn disable(project: &Path, password: &SecretString) -> Result<()> {
    let _guard = write_lock();
    let files = read_all_raw(project)?;
    if !is_on(project, &files) {
        return Err(Error::EncryptionOff);
    }
    for key in verify(password, &files)? {
        keyring::insert_for_opening(project, key);
    }
    // Open everything first, so a damaged value stops before any write.
    let opened =
        files.iter().map(|(file, env)| Ok((file, open_all_held(project, env)?))).collect::<Result<Vec<_>>>()?;
    for ((file, plain), (_, stored)) in opened.into_iter().zip(&files) {
        if &plain != stored {
            write_raw(project, file, &plain)?;
        }
    }
    set_settings(project, None)?;
    keyring::lock(project);
    Ok(())
}

/// Seal every value again under `new`. Values interrupted half way stay
/// under the old password and are brought over at the next unlock.
pub fn change_password(project: &Path, current: &SecretString, new: &SecretString) -> Result<()> {
    new.check_length()?;
    let _guard = write_lock();
    let files = read_all_raw(project)?;
    if !is_on(project, &files) {
        return Err(Error::EncryptionOff);
    }
    for key in verify(current, &files)? {
        keyring::insert_for_opening(project, key);
    }
    let key = keyring::insert(project, DataKey::generate(new)?);
    for (file, env) in &files {
        if !env.is_empty() {
            write_raw(project, file, &seal_all(&key, &open_all_held(project, env)?)?)?;
        }
    }
    Ok(())
}

/// For a forgotten password: remove every sealed value (plain ones stay)
/// and turn encryption off.
pub fn reset(project: &Path) -> Result<()> {
    let _guard = write_lock();
    for (file, mut env) in read_all_raw(project)? {
        let before = env.len();
        env.retain(|_, value| !is_sealed(value));
        if env.len() != before {
            write_raw(project, &file, &env)?;
        }
    }
    set_settings(project, None)?;
    keyring::lock(project);
    Ok(())
}

pub fn set_auto_lock(project: &Path, minutes: Option<u32>) -> Result<()> {
    let _guard = write_lock();
    if !is_on(project, &read_all_raw(project)?) {
        return Err(Error::EncryptionOff);
    }
    set_settings(project, Some(SecretSettings { auto_lock_minutes: minutes }))
}

/// Forget the project's keys.
pub fn lock(project: &Path) {
    keyring::lock(project);
}

/// Forget every project's keys.
pub fn lock_all() {
    keyring::lock_all();
}

/// Unlock with `password`.
///
/// - Nothing sealed yet: there is nothing to check the password against,
///   so it becomes the password (the UI asks for it twice).
/// - Otherwise it must open some of the values, or it's
///   [`Error::WrongPassword`]. If values remain under another password (a
///   password change was interrupted), the project stays locked until that
///   one is given too.
/// - Once every value opens, plain values and any under another password
///   are sealed again under this one.
pub fn unlock(project: &Path, password: &SecretString) -> Result<UnlockReport> {
    let _guard = write_lock();
    let files = read_all_raw(project)?;
    if !is_on(project, &files) {
        return Err(Error::EncryptionOff);
    }
    let samples = sealed_by_salt(&files)?;
    let key = if samples.is_empty() {
        password.check_length()?;
        keyring::insert(project, DataKey::generate(password)?)
    } else {
        let (keys, _) = derive_for(password, &files)?;
        let mut keys = keys.into_iter();
        let Some(first) = keys.next() else { return Err(Error::WrongPassword) };
        keys.for_each(|key| keyring::insert_for_opening(project, key));
        keyring::insert(project, first)
    };
    if samples.keys().any(|salt| keyring::for_salt(project, salt).is_none()) {
        return Ok(UnlockReport { sealed_plain_values: 0 });
    }
    let mut sealed_plain_values = 0;
    for (file, env) in &files {
        let stale = |(name, value): (&String, &String)| {
            !is_sealed(value) || super::cipher::salt_of(name, value).is_ok_and(|salt| salt != key.salt())
        };
        if !env.iter().any(stale) {
            continue;
        }
        sealed_plain_values += u32::try_from(env.values().filter(|v| !is_sealed(v)).count()).unwrap_or(u32::MAX);
        write_raw(project, file, &seal_all(&key, &open_all_held(project, env)?)?)?;
    }
    Ok(UnlockReport { sealed_plain_values })
}

/// Unlock without writing anything. For the CLI, which must never rewrite
/// local files in CI. Values no key opens stay locked.
pub fn unlock_read_only(project: &Path, password: &SecretString) -> Result<()> {
    let files = read_all_raw(project)?;
    let (keys, _) = derive_for(password, &files)?;
    if keys.is_empty() && !sealed_by_salt(&files)?.is_empty() {
        return Err(Error::WrongPassword);
    }
    for key in keys {
        keyring::insert_for_opening(project, key);
    }
    Ok(())
}

/// Whether any secrets file holds a sealed value, so sending would need
/// the password (the CLI asks only then).
pub fn has_sealed_values(project: &Path) -> Result<bool> {
    Ok(!sealed_by_salt(&read_all_raw(project)?)?.is_empty())
}
