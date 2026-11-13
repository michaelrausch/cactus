//! A build without the `encryption` feature: the same API, always off.
//! Keep the signatures in step with the encryption build, or one feature
//! combination stops compiling.

use std::path::Path;

use super::store::read_all_raw;
use super::{SecretsInfo, SecretsStatus, UnlockReport, is_sealed};
use crate::{Error, Result};

/// A password the UI or CLI passed in; unused in this build.
pub struct SecretString(String);

impl SecretString {
    pub fn new(password: String) -> Self {
        Self(password)
    }
}

impl From<String> for SecretString {
    fn from(password: String) -> Self {
        Self(password)
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.0.clear();
    }
}

pub fn info(project: &Path) -> Result<SecretsInfo> {
    Ok(SecretsInfo {
        status: SecretsStatus::Off,
        auto_lock_minutes: None,
        has_sealed_values: has_sealed_values(project)?,
    })
}

pub fn status(_project: &Path) -> Result<SecretsStatus> {
    Ok(SecretsStatus::Off)
}

/// Sealed values from a build with encryption; this one can't open them.
pub fn has_sealed_values(project: &Path) -> Result<bool> {
    Ok(read_all_raw(project)?.iter().flat_map(|(_, env)| env.values()).any(|value| is_sealed(value)))
}

pub fn enable(_project: &Path, _password: &SecretString, _auto_lock_minutes: Option<u32>) -> Result<()> {
    Err(Error::EncryptionOff)
}

pub fn disable(_project: &Path, _password: &SecretString) -> Result<()> {
    Err(Error::EncryptionOff)
}

pub fn change_password(_project: &Path, _current: &SecretString, _new: &SecretString) -> Result<()> {
    Err(Error::EncryptionOff)
}

pub fn reset(_project: &Path) -> Result<()> {
    Err(Error::EncryptionOff)
}

pub fn set_auto_lock(_project: &Path, _minutes: Option<u32>) -> Result<()> {
    Err(Error::EncryptionOff)
}

pub fn unlock(_project: &Path, _password: &SecretString) -> Result<UnlockReport> {
    Err(Error::EncryptionOff)
}

pub fn unlock_read_only(_project: &Path, _password: &SecretString) -> Result<()> {
    Err(Error::EncryptionOff)
}

pub fn lock(_project: &Path) {}

pub fn lock_all() {}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::env::{self, Env, EnvFile};

    #[test]
    fn plain_text_always_and_sealed_values_never_used() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        assert!(matches!(enable(p, &"password1".to_string().into(), None), Err(Error::EncryptionOff)));
        env::save(p, &EnvFile::Secrets, &Env::from([("a".into(), "1".into())])).unwrap();
        assert_eq!(fs::read_to_string(p.join(".cactus/secrets.json")).unwrap(), "{\n  \"a\": \"1\"\n}\n");
        assert_eq!(info(p).unwrap().status, SecretsStatus::Off);

        fs::write(p.join(".cactus/secrets.json"), r#"{"a": "enc:v1:salt:body"}"#).unwrap();
        assert!(matches!(env::load(p, &EnvFile::Secrets), Err(Error::SecretsLocked)));
        assert!(crate::secrets::locked_for(p, &[crate::secrets::SecretFile::Project]));
        assert!(has_sealed_values(p).unwrap());
    }
}
