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

use crate::{Result, ScenarioPath};

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
/// History files are flat, so folder separators are encoded in the name.
const HISTORY_PATH_SEPARATOR: &str = "__";
const HISTORY_EXTENSION: &str = ".json";

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
        fs::create_dir_all(&dir)?;
        fs::write(dir.join(".gitignore"), WORKSPACE_GITIGNORE)?;
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

/// `a/b.yml` → `.cactus/history/a__b.json`.
pub fn history_file(project: &Path, scenario: &ScenarioPath) -> PathBuf {
    let flat = scenario.as_str().replace('/', HISTORY_PATH_SEPARATOR);
    let stem = flat.strip_suffix(REQUEST_EXTENSION).unwrap_or(&flat);
    workspace_dir(project).join(HISTORY_DIR).join(format!("{stem}{HISTORY_EXTENSION}"))
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
        let path = history_file(Path::new("/p"), &ScenarioPath::parse("a/b.yml").unwrap());
        assert_eq!(path, Path::new("/p/.cactus/history/a__b.json"));
    }
}
