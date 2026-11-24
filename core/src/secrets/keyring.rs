//! The keys of unlocked projects, held for the life of the process.
//!
//! A process-wide keyring rather than a key passed through every call: the
//! key must reach env loading, the runner, flows, the matrix and the
//! scheduler, and threading it through would change all their signatures.
//! Keys are shared as `Arc`, so a send in progress keeps its key if the
//! project is locked meanwhile; a key is wiped when the last one drops.
//!
//! A project normally has one salt and so one key. More than one happens
//! only when changing the password was interrupted; each key is kept by
//! salt until unlocking brings every value back under one.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};

use super::cipher::DataKey;

#[derive(Default)]
struct ProjectKeys {
    by_salt: HashMap<String, Arc<DataKey>>,
    /// The key new values are sealed with.
    current: Option<Arc<DataKey>>,
}

static KEYRING: LazyLock<Mutex<HashMap<PathBuf, ProjectKeys>>> = LazyLock::new(Mutex::default);

/// Projects are keyed by canonical path, so `/p` and `/p/.` are one project.
fn canonical(project: &Path) -> PathBuf {
    fs::canonicalize(project).unwrap_or_else(|_| project.to_path_buf())
}

fn keyring() -> MutexGuard<'static, HashMap<PathBuf, ProjectKeys>> {
    KEYRING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The key that opens values sealed under `salt`, if unlocked.
pub fn for_salt(project: &Path, salt: &str) -> Option<Arc<DataKey>> {
    keyring().get(&canonical(project))?.by_salt.get(salt).cloned()
}

/// The key new values are sealed with, if unlocked.
pub fn current(project: &Path) -> Option<Arc<DataKey>> {
    keyring().get(&canonical(project))?.current.clone()
}

/// Hold `key`, and seal new values with it.
pub fn insert(project: &Path, key: DataKey) -> Arc<DataKey> {
    let key = Arc::new(key);
    let mut keyring = keyring();
    let keys = keyring.entry(canonical(project)).or_default();
    keys.by_salt.insert(key.salt().to_string(), Arc::clone(&key));
    keys.current = Some(Arc::clone(&key));
    key
}

/// Hold `key` for opening values only.
pub fn insert_for_opening(project: &Path, key: DataKey) {
    let key = Arc::new(key);
    keyring().entry(canonical(project)).or_default().by_salt.insert(key.salt().to_string(), key);
}

/// Forget the project's keys. Sends already holding one finish.
pub fn lock(project: &Path) {
    keyring().remove(&canonical(project));
}

/// Forget every key, e.g. when another project opens.
pub fn lock_all() {
    keyring().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::cipher::{open_value, seal_value};

    #[test]
    fn canonical_paths_share_a_key_and_held_keys_survive_lock() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let key = insert(p, DataKey::generate(&"password1".to_string().into()).unwrap());
        let held = current(&p.join(".")).expect("same project");
        assert!(for_salt(p, key.salt()).is_some());
        let sealed = seal_value(&held, "a", "v").unwrap();
        lock(p);
        assert!(current(p).is_none() && for_salt(p, key.salt()).is_none());
        assert_eq!(open_value(&held, "a", &sealed).unwrap().unwrap(), "v");
    }
}
