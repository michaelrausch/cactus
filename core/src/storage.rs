//! File persistence shared by every on-disk store.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Result;

/// Appended to a file name while it is being written; the watcher ignores it.
const TEMP_SUFFIX: &str = ".tmp";

/// Read a JSON file, treating a missing file as `T::default()`.
pub fn read_json<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    match fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}

/// Read a JSON file whose loss is harmless (caches, local state): any
/// failure, including corruption, yields `T::default()`.
pub fn read_json_lenient<T: DeserializeOwned + Default>(path: &Path) -> T {
    read_json(path).unwrap_or_default()
}

/// Pretty-printed write via a temp file and rename, so readers and file
/// watchers never observe a half-written file.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write_atomic(path, &bytes)
}

/// Write via a temp file and rename, so readers and file watchers never
/// observe a half-written file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(TEMP_SUFFIX);
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
