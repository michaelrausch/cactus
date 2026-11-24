//! Reading and writing the YAML files of a collection.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::emit::to_yaml;
use super::patch::patch;
use crate::storage::write_atomic;
use crate::{Error, Result};

pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    parse(&fs::read_to_string(path).map_err(Error::file(path))?)
}

/// Like [`read`], with a missing file read as `None`.
pub fn read_if_exists<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match fs::read_to_string(path) {
        Ok(text) => parse(&text).map(Some),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::file(path)(e)),
    }
}

/// An empty file reads like an empty mapping.
fn parse<T: DeserializeOwned>(text: &str) -> Result<T> {
    if text.trim().is_empty() {
        return Ok(serde_yaml_ng::from_str("{}")?);
    }
    Ok(serde_yaml_ng::from_str(text)?)
}

/// Save `value` to `path`. An existing file is edited in place: only what
/// changed since it was last read into `T` is rewritten, so comments and
/// keys `T` doesn't know survive. A new file is written from scratch.
pub fn write<T: Serialize + DeserializeOwned>(path: &Path, value: &T) -> Result<()> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return write_atomic(path, to_yaml(&serde_json::to_value(value)?)?.as_bytes());
        }
        Err(e) => return Err(Error::file(path)(e)),
    };
    let unpatchable = |reason: String| Error::Unpatchable { path: path.display().to_string(), reason };
    // A file that no longer parses is the user's to fix; overwriting it
    // would lose their text.
    let old = serde_json::to_value(parse::<T>(&text).map_err(|e| unpatchable(e.to_string()))?)?;
    let new = serde_json::to_value(value)?;
    if old == new {
        return Ok(());
    }
    let patched = patch(&text, &old, &new).map_err(unpatchable)?;
    // Never write an edit that doesn't read back as what was saved.
    let reread = parse::<T>(&patched).map(|v| serde_json::to_value(v).unwrap_or(Value::Null));
    if !reread.is_ok_and(|reread| reread == new) {
        return Err(unpatchable("the edited file would not read back as saved".into()));
    }
    write_atomic(path, patched.as_bytes())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Doc {
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
    }

    #[test]
    fn writes_new_and_patches_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("d.yml");
        write(&path, &Doc { name: "a".into(), tags: vec![] }).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "name: a\n");

        fs::write(&path, "# mine\nname: a # inline\nextra:\n  keep: 1\n").unwrap();
        write(&path, &Doc { name: "b".into(), tags: vec!["t".into()] }).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "# mine\nname: b # inline\ntags:\n  - t\nextra:\n  keep: 1\n");
        assert_eq!(read::<Doc>(&path).unwrap().tags, ["t"]);
    }

    #[test]
    fn refuses_to_overwrite_a_broken_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("d.yml");
        fs::write(&path, "name: [unclosed\n").unwrap();
        assert!(matches!(write(&path, &Doc { name: "b".into(), tags: vec![] }), Err(Error::Unpatchable { .. })));
        assert_eq!(fs::read_to_string(&path).unwrap(), "name: [unclosed\n");
    }
}
