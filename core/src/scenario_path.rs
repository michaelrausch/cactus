use std::borrow::Cow;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const SEPARATOR: char = '/';
/// Accepted on input on every platform, so a path typed on Windows means the
/// same everywhere; it is stored as [`SEPARATOR`].
const WINDOWS_SEPARATOR: char = '\\';
/// Marks a drive (`C:`) or an NTFS alternate data stream (`a.yml:x`), either
/// of which would make a joined path leave the project on Windows.
const DRIVE_MARKER: char = ':';

/// A request file's location relative to the project root, using `/` separators.
/// It can never point outside the project, so it is safe to join onto it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "specta", derive(specta::Type), specta(transparent))]
pub struct ScenarioPath(String);

impl ScenarioPath {
    /// Accepts `\` as well as `/` between folders. Every part must be a plain
    /// name: no empty parts, `.`, `..`, roots or drives.
    pub fn parse(relative: impl Into<String>) -> Result<Self> {
        let relative = relative.into();
        let normalized = relative.replace(WINDOWS_SEPARATOR, &SEPARATOR.to_string());
        let plain_parts = normalized.split(SEPARATOR).all(is_plain_name);
        // Also ask the platform, which knows its own kinds of prefix.
        let only_normal_parts = Path::new(&normalized).components().all(|c| matches!(c, Component::Normal(_)));
        if !plain_parts || !only_normal_parts {
            return Err(Error::InvalidScenarioPath(relative));
        }
        Ok(Self(normalized))
    }

    /// `file_name` inside folder `dir`, where `""` is the project root.
    pub fn in_dir(dir: &str, file_name: &str) -> Result<Self> {
        let dir = normalize_dir(dir);
        if dir.is_empty() { Self::parse(file_name) } else { Self::parse(format!("{dir}{SEPARATOR}{file_name}")) }
    }

    /// Build from a path below `project`, normalising platform separators.
    pub fn from_absolute(project: &Path, path: &Path) -> Option<Self> {
        let relative = path.strip_prefix(project).ok()?.to_string_lossy().replace('\\', "/");
        Self::parse(relative).ok()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The containing folder, or `""` at the project root.
    pub fn dir(&self) -> &str {
        self.0.rsplit_once(SEPARATOR).map_or("", |(dir, _)| dir)
    }

    pub fn file_name(&self) -> &str {
        self.0.rsplit_once(SEPARATOR).map_or(&self.0, |(_, name)| name)
    }

    /// Whether this path lies under folder `dir`; everything is within `""`.
    pub fn is_within(&self, dir: &str) -> bool {
        let dir = normalize_dir(dir);
        dir.is_empty() || self.0.strip_prefix(&*dir).is_some_and(|rest| rest.starts_with(SEPARATOR))
    }

    pub fn resolve(&self, project: &Path) -> PathBuf {
        project.join(&self.0)
    }
}

fn is_plain_name(part: &str) -> bool {
    !part.is_empty() && part != "." && part != ".." && !part.contains(DRIVE_MARKER)
}

/// A folder as given by a user or the UI: either separator, maybe a trailing one.
fn normalize_dir(dir: &str) -> Cow<'_, str> {
    let dir = dir.trim_end_matches([SEPARATOR, WINDOWS_SEPARATOR]);
    if dir.contains(WINDOWS_SEPARATOR) {
        Cow::Owned(dir.replace(WINDOWS_SEPARATOR, &SEPARATOR.to_string()))
    } else {
        Cow::Borrowed(dir)
    }
}

impl TryFrom<String> for ScenarioPath {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::parse(value)
    }
}

impl From<ScenarioPath> for String {
    fn from(path: ScenarioPath) -> Self {
        path.0
    }
}

impl fmt::Display for ScenarioPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_escape() {
        assert!(ScenarioPath::parse("../x.yml").is_err());
        assert!(ScenarioPath::parse("/etc/passwd").is_err());
        assert!(ScenarioPath::parse("").is_err());
        assert!(ScenarioPath::parse("a/b.yml").is_ok());
        for bad in [
            "a//b.yml",
            "a/",
            "./a.yml",
            "a/../b.yml",
            "C:/x.yml",
            "C:x.yml",
            "a.yml:stream",
            "\\\\srv\\share\\a.yml",
            "\\a.yml",
        ] {
            assert!(ScenarioPath::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn normalizes_windows_separators() {
        let path = ScenarioPath::parse("auth\\admin\\login.yml").unwrap();
        assert_eq!(path.as_str(), "auth/admin/login.yml");
        assert_eq!((path.dir(), path.file_name()), ("auth/admin", "login.yml"));
        assert!(path.is_within("auth\\"));
        assert!(path.is_within("auth\\admin"));
        assert_eq!(ScenarioPath::in_dir("auth\\", "x.yml").unwrap().as_str(), "auth/x.yml");
    }

    #[test]
    fn splits_dir_and_name() {
        let path = ScenarioPath::parse("a/b/c.yml").unwrap();
        assert_eq!((path.dir(), path.file_name()), ("a/b", "c.yml"));
        let root = ScenarioPath::parse("c.yml").unwrap();
        assert_eq!((root.dir(), root.file_name()), ("", "c.yml"));
        assert_eq!(ScenarioPath::in_dir("a/", "c.yml").unwrap().as_str(), "a/c.yml");
    }

    #[test]
    fn within_matches_whole_folders() {
        let path = ScenarioPath::parse("auth/login.yml").unwrap();
        assert!(path.is_within(""));
        assert!(path.is_within("auth"));
        assert!(path.is_within("auth/"));
        assert!(!path.is_within("aut"));
        assert!(!path.is_within("other"));
    }
}
