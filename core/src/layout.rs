//! Where cactus keeps things inside a project folder.
//!
//! Shared, committed files live at the root (`env.json`, `env.<name>.json`,
//! scenarios). Machine-local state lives in `.cactus/`, which ignores itself
//! in git.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Result, ScenarioPath};

pub const WORKSPACE_DIR: &str = ".cactus";
const WORKSPACE_GITIGNORE: &str = "*\n";

pub const BASE_ENV_FILE: &str = "env.json";
const NAMED_ENV_PREFIX: &str = "env.";
const JSON_EXTENSION: &str = ".json";

const SECRETS_FILE: &str = "secrets.json";
const CAPTURED_FILE: &str = "captured.json";
const STATE_FILE: &str = "state.json";
const HISTORY_DIR: &str = "history";
/// History files are flat, so folder separators are encoded in the name.
const HISTORY_PATH_SEPARATOR: &str = "__";

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

pub fn history_file(project: &Path, scenario: &ScenarioPath) -> PathBuf {
    workspace_dir(project).join(HISTORY_DIR).join(scenario.as_str().replace('/', HISTORY_PATH_SEPARATOR))
}

pub fn named_env_file_name(name: &str) -> String {
    format!("{NAMED_ENV_PREFIX}{name}{JSON_EXTENSION}")
}

/// `"env.dev.json"` → `Some("dev")`.
pub fn named_env_from_file_name(file_name: &str) -> Option<&str> {
    file_name.strip_prefix(NAMED_ENV_PREFIX)?.strip_suffix(JSON_EXTENSION)
}

/// `env.json` or `env.<name>.json`; only meaningful at the project root.
pub fn is_env_file(file_name: &str) -> bool {
    file_name == BASE_ENV_FILE || named_env_from_file_name(file_name).is_some()
}

pub fn is_json_file(file_name: &str) -> bool {
    file_name.ends_with(JSON_EXTENSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_file_names() {
        assert!(is_env_file("env.json"));
        assert!(is_env_file("env.dev.json"));
        assert!(!is_env_file("envy.json"));
        assert_eq!(named_env_from_file_name(&named_env_file_name("prod")), Some("prod"));
    }

    #[test]
    fn history_file_flattens_folders() {
        let path = history_file(Path::new("/p"), &ScenarioPath::parse("a/b.json").unwrap());
        assert_eq!(path, Path::new("/p/.cactus/history/a__b.json"));
    }
}
