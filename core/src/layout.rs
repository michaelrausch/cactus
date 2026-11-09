//! Where cactus keeps things inside a project folder.
//!
//! A project is an OpenCollection stored as files, the layout Bruno uses:
//!
//! - `opencollection.yml`: the collection, its defaults and shared variables
//! - `environments/<name>.yml`: one named environment each
//! - `<folder>/folder.yml`: a folder's defaults
//! - every other `*.yml`: one request
//!
//! Machine-local state lives in `.cactus/`, which ignores itself in git.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Error, ProjectPath, Result};

pub const COLLECTION_FILE: &str = "opencollection.yml";
pub const FOLDER_FILE: &str = "folder.yml";
pub const ENVIRONMENTS_DIR: &str = "environments";
pub const REQUEST_EXTENSION: &str = ".yml";

pub const WORKSPACE_DIR: &str = ".cactus";
const WORKSPACE_GITIGNORE: &str = "*\n";

const SECRETS_FILE: &str = "secrets.json";
const CAPTURED_FILE: &str = "captured.json";
const STATE_FILE: &str = "state.json";
const HISTORY_DIR: &str = "history";
const HISTORY_EXTENSION: &str = ".json";
/// Appended to a file name while it is being written; the watcher ignores it.
pub const TEMP_FILE_SUFFIX: &str = ".tmp";

pub fn collection_file(project: &Path) -> PathBuf {
    project.join(COLLECTION_FILE)
}

/// `folder.yml` of folder `dir`, relative to the project.
pub fn folder_file(project: &Path, dir: &str) -> PathBuf {
    project.join(dir).join(FOLDER_FILE)
}

pub fn environments_dir(project: &Path) -> PathBuf {
    project.join(ENVIRONMENTS_DIR)
}

pub fn environment_file(project: &Path, name: &str) -> PathBuf {
    environments_dir(project).join(format!("{name}{REQUEST_EXTENSION}"))
}

/// `"dev.yml"` → `Some("dev")`.
pub fn environment_from_file_name(file_name: &str) -> Option<&str> {
    file_name.strip_suffix(REQUEST_EXTENSION)
}

/// Whether the file at `rel_path` (project-relative, `/`-separated) holds a
/// request rather than collection, folder or environment settings.
pub fn is_request_file(rel_path: &str) -> bool {
    let file_name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    let in_environments = rel_path.strip_prefix(ENVIRONMENTS_DIR).is_some_and(|rest| rest.starts_with('/'));
    rel_path.ends_with(REQUEST_EXTENSION) && rel_path != COLLECTION_FILE && file_name != FOLDER_FILE && !in_environments
}

pub fn workspace_dir(project: &Path) -> PathBuf {
    project.join(WORKSPACE_DIR)
}

/// Create `.cactus/` with a `.gitignore` that ignores everything in it.
pub fn ensure_workspace(project: &Path) -> Result<PathBuf> {
    let dir = workspace_dir(project);
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(Error::file(&dir))?;
        let gitignore = dir.join(".gitignore");
        fs::write(&gitignore, WORKSPACE_GITIGNORE).map_err(Error::file(&gitignore))?;
    }
    Ok(dir)
}

pub fn state_file(project: &Path) -> PathBuf {
    workspace_dir(project).join(STATE_FILE)
}

pub fn secrets_file(project: &Path) -> PathBuf {
    workspace_dir(project).join(SECRETS_FILE)
}

pub fn captured_file(project: &Path) -> PathBuf {
    workspace_dir(project).join(CAPTURED_FILE)
}

/// `a/b.yml` → `.cactus/history/a%2Fb.json`. History files are flat, so the
/// folder separator is percent-encoded, and `%` itself too, which keeps
/// every request's file distinct (`a/b.yml` and `a%2Fb.yml` included).
pub fn history_file(project: &Path, scenario: &ProjectPath) -> PathBuf {
    let path = scenario.as_str();
    let stem = path.strip_suffix(REQUEST_EXTENSION).unwrap_or(path).replace('%', "%25").replace('/', "%2F");
    workspace_dir(project).join(HISTORY_DIR).join(format!("{stem}{HISTORY_EXTENSION}"))
}

/// Each folder on the way down to `dir` (`""` is the root), as its path
/// and its name: `"a/b"` → `("a", "a")`, `("a/b", "b")`.
pub fn folder_prefixes(dir: &str) -> impl Iterator<Item = (String, &str)> {
    let mut path = String::new();
    dir.split('/').filter(|part| !part.is_empty()).map(move |name| {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(name);
        (path.clone(), name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_files() {
        assert!(is_request_file("login.yml"));
        assert!(is_request_file("auth/login.yml"));
        assert!(is_request_file("auth/environments/x.yml"));
        assert!(!is_request_file("opencollection.yml"));
        assert!(!is_request_file("auth/folder.yml"));
        assert!(!is_request_file("environments/dev.yml"));
        assert!(!is_request_file("notes.json"));
        assert_eq!(environment_from_file_name("dev.yml"), Some("dev"));
    }

    #[test]
    fn history_file_flattens_folders() {
        let file = |rel: &str| history_file(Path::new("/p"), &ProjectPath::parse(rel).unwrap());
        assert_eq!(file("a/b.yml"), Path::new("/p/.cactus/history/a%2Fb.json"));
        assert_ne!(file("a/b.yml"), file("a__b.yml"));
        assert_ne!(file("a/b.yml"), file("a%2Fb.yml"));
    }

    #[test]
    fn folder_prefixes_walk_down() {
        let prefixes: Vec<_> = folder_prefixes("a/b/").collect();
        assert_eq!(prefixes, [("a".to_string(), "a"), ("a/b".to_string(), "b")]);
        assert_eq!(folder_prefixes("").count(), 0);
    }
}
