//! Where cactus keeps things inside a project folder.
//!
//! A project is an OpenCollection stored as files, the layout Bruno uses:
//!
//! - `opencollection.yml`: the collection, its defaults and shared variables
//! - `environments/<name>.yml`: one named environment each
//! - `<folder>/folder.yml`: a folder's defaults
//! - `critters/<id>/critter.yml` and `critters/<id>/flows/<flow>.yml`: personas
//! - every other `*.yml`: one request
//!
//! Machine-local state lives in `.cactus/`, which ignores itself in git.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Error, ProjectPath, Result};

pub const COLLECTION_FILE: &str = "opencollection.yml";
pub const FOLDER_FILE: &str = "folder.yml";
pub const ENVIRONMENTS_DIR: &str = "environments";
pub const CRITTERS_DIR: &str = "critters";
pub const CRITTER_FILE: &str = "critter.yml";
pub const FLOWS_DIR: &str = "flows";
pub const REQUEST_EXTENSION: &str = ".yml";

pub const WORKSPACE_DIR: &str = ".cactus";
const WORKSPACE_GITIGNORE: &str = "*\n";

// Secrets files (`secrets.json` here and in each critter's folder) and
// `vault.json` are only read and written by `crate::secrets`.
const CAPTURED_FILE: &str = "captured.json";
const STATE_FILE: &str = "state.json";
const HISTORY_DIR: &str = "history";
const HISTORY_EXTENSION: &str = ".json";
const CRITTER_CONTEXT_FILE: &str = "context.json";
const RUNS_DIR: &str = "runs";
const RUN_LOG_EXTENSION: &str = ".json";
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
    let in_top_level = |dir: &str| rel_path.strip_prefix(dir).is_some_and(|rest| rest.starts_with('/'));
    rel_path.ends_with(REQUEST_EXTENSION)
        && rel_path != COLLECTION_FILE
        && file_name != FOLDER_FILE
        && !in_top_level(ENVIRONMENTS_DIR)
        && !in_top_level(CRITTERS_DIR)
}

pub fn critters_dir(project: &Path) -> PathBuf {
    project.join(CRITTERS_DIR)
}

/// `critters/<id>/`. `id` must already be validated.
pub fn critter_dir(project: &Path, id: &str) -> PathBuf {
    critters_dir(project).join(id)
}

pub fn critter_file(project: &Path, id: &str) -> PathBuf {
    critter_dir(project, id).join(CRITTER_FILE)
}

pub fn flows_dir(project: &Path, id: &str) -> PathBuf {
    critter_dir(project, id).join(FLOWS_DIR)
}

pub fn flow_file(project: &Path, id: &str, flow: &str) -> PathBuf {
    flows_dir(project, id).join(format!("{flow}{REQUEST_EXTENSION}"))
}

/// `"login.yml"` → `Some("login")`.
pub fn flow_from_file_name(file_name: &str) -> Option<&str> {
    file_name.strip_suffix(REQUEST_EXTENSION)
}

/// `.cactus/critters/<id>/`: the critter's secrets, context and run logs.
pub fn critter_workspace_dir(project: &Path, id: &str) -> PathBuf {
    workspace_dir(project).join(CRITTERS_DIR).join(id)
}

pub fn critter_context_file(project: &Path, id: &str) -> PathBuf {
    critter_workspace_dir(project, id).join(CRITTER_CONTEXT_FILE)
}

/// Run logs live with the critter's other local files, so renaming or
/// deleting a critter takes them along.
pub fn runs_dir(project: &Path, id: &str) -> PathBuf {
    critter_workspace_dir(project, id).join(RUNS_DIR)
}

pub fn run_log_file(project: &Path, id: &str, flow: &str) -> PathBuf {
    runs_dir(project, id).join(format!("{flow}{RUN_LOG_EXTENSION}"))
}

/// Which critter, and which of its flows, a project-relative path belongs
/// to: `critters/bob/critter.yml` → `("bob", None)`,
/// `critters/bob/flows/login.yml` → `("bob", Some("login"))`. Other files
/// under a critter's folder count as the critter itself.
pub fn critter_from_path(rel_path: &str) -> Option<(&str, Option<&str>)> {
    let rest = rel_path.strip_prefix(CRITTERS_DIR)?.strip_prefix('/')?;
    let (id, rest) = rest.split_once('/').unwrap_or((rest, ""));
    if id.is_empty() {
        return None;
    }
    let flow = rest.strip_prefix(FLOWS_DIR).and_then(|r| r.strip_prefix('/')).and_then(flow_from_file_name);
    Some((id, flow.filter(|flow| !flow.is_empty() && !flow.contains('/'))))
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
        assert!(!is_request_file("critters/bob/critter.yml"));
        assert!(!is_request_file("critters/bob/flows/login.yml"));
        assert!(is_request_file("api/critters/x.yml"));
        assert!(is_request_file("critters.yml"));
        assert!(!is_request_file("notes.json"));
        assert_eq!(environment_from_file_name("dev.yml"), Some("dev"));
    }

    #[test]
    fn critter_paths() {
        assert_eq!(critter_from_path("critters/bob/critter.yml"), Some(("bob", None)));
        assert_eq!(critter_from_path("critters/bob/flows/login.yml"), Some(("bob", Some("login"))));
        assert_eq!(critter_from_path("critters/bob"), Some(("bob", None)));
        assert_eq!(critter_from_path("critters/bob/flows/a/b.yml"), Some(("bob", None)));
        assert_eq!(critter_from_path("critters/"), None);
        assert_eq!(critter_from_path("crittersx/bob"), None);
        assert_eq!(critter_from_path("auth/login.yml"), None);
        let p = Path::new("/p");
        assert_eq!(run_log_file(p, "bob", "health"), Path::new("/p/.cactus/critters/bob/runs/health.json"));
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
