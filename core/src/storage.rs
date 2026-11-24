//! File persistence shared by every on-disk store.

use std::ffi::OsString;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Result;

/// Appended to a file name while it is being written; the watcher ignores it.
const TEMP_SUFFIX: &str = ".tmp";
/// On Windows a rename fails while another process (antivirus, the search
/// indexer, an editor) briefly has the target open, so it is retried for up
/// to about a second. Elsewhere the same error is a real permission problem.
const RENAME_ATTEMPTS: u32 = if cfg!(windows) { 10 } else { 1 };
const RENAME_RETRY_DELAY: Duration = Duration::from_millis(100);

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
/// The temp file is removed if the write fails.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp: OsString = path.as_os_str().to_owned();
    tmp.push(TEMP_SUFFIX);
    let tmp = Path::new(&tmp);
    let written = fs::write(tmp, bytes).and_then(|()| rename_with_retry(tmp, path));
    if written.is_err() {
        let _ = fs::remove_file(tmp);
    }
    Ok(written?)
}

fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    let mut attempt = 1;
    loop {
        match fs::rename(from, to) {
            Err(e) if e.kind() == ErrorKind::PermissionDenied && attempt < RENAME_ATTEMPTS => {
                attempt += 1;
                thread::sleep(RENAME_RETRY_DELAY);
            }
            result => return result,
        }
    }
}

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_replaces_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.yml");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_write_removes_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        // Renaming a file over a directory fails on every platform.
        let path = dir.path().join("taken");
        fs::create_dir(&path).unwrap();
        assert!(write_atomic(&path, b"x").is_err());
        assert!(!dir.path().join("taken.tmp").exists());
    }
}
