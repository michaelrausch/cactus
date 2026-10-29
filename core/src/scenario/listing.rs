//! Discovering scenario files in a project folder.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

use super::model::default_method;
use crate::{Error, Result, ScenarioPath, layout};

/// Folders that never hold scenarios but can be huge.
const IGNORED_DIRS: &[&str] = &["node_modules", "target", "dist", "build", "vendor", "__pycache__", "venv"];
/// Opening a folder like `~` by mistake should stay fast.
const MAX_WALK_DEPTH: usize = 6;
/// Anything bigger is data, not a scenario, and isn't worth parsing for the list.
const MAX_SCENARIO_BYTES: u64 = 1024 * 1024;
/// Shown as the method of files that aren't valid scenarios.
const UNPARSEABLE_METHOD: &str = "???";

/// Just enough of a scenario to render the sidebar.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ScenarioEntry {
    pub rel_path: ScenarioPath,
    pub title: String,
    pub method: String,
}

/// The fields of a scenario file the listing reads.
#[derive(Deserialize)]
struct ScenarioHeader {
    #[serde(default)]
    title: String,
    #[serde(default = "default_method")]
    method: String,
}

impl ScenarioHeader {
    fn read(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_else(|| Self { title: String::new(), method: UNPARSEABLE_METHOD.into() })
    }
}

/// Walk the project in name order, skipping dot-folders, [`IGNORED_DIRS`]
/// and unreadable entries.
pub fn walk(project: &Path) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(project)
        .max_depth(MAX_WALK_DEPTH)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !is_skipped(e))
        .filter_map(std::result::Result::ok)
}

fn is_skipped(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    name.starts_with('.') || (entry.file_type().is_dir() && IGNORED_DIRS.contains(&&*name))
}

/// Every scenario in the project, sorted by path. Unparseable JSON files are
/// included so the user can see and fix them.
pub fn list(project: &Path) -> Result<Vec<ScenarioEntry>> {
    if !project.is_dir() {
        return Err(Error::NotAFolder(project.to_path_buf()));
    }
    Ok(walk(project).filter(is_scenario_candidate).filter_map(|e| read_entry(project, &e)).collect())
}

fn is_scenario_candidate(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    let is_root_env_file = entry.depth() == 1 && layout::is_env_file(&name);
    let too_big = entry.metadata().is_ok_and(|m| m.len() > MAX_SCENARIO_BYTES);
    entry.file_type().is_file() && layout::is_json_file(&name) && !is_root_env_file && !too_big
}

fn read_entry(project: &Path, entry: &DirEntry) -> Option<ScenarioEntry> {
    let rel_path = ScenarioPath::from_absolute(project, entry.path())?;
    let header = ScenarioHeader::read(entry.path());
    let title = if header.title.is_empty() { fallback_title(&rel_path) } else { header.title };
    Some(ScenarioEntry { rel_path, title, method: header.method })
}

fn fallback_title(path: &ScenarioPath) -> String {
    path.as_str().trim_end_matches(".json").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_skips_env_and_dotfiles() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        fs::create_dir_all(p.join("auth")).unwrap();
        fs::create_dir_all(p.join(".cactus")).unwrap();
        fs::create_dir_all(p.join("node_modules/pkg")).unwrap();
        fs::write(p.join("node_modules/pkg/package.json"), "{}").unwrap();
        fs::write(p.join("env.json"), "{}").unwrap();
        fs::write(p.join("env.dev.json"), "{}").unwrap();
        fs::write(p.join(".cactus/state.json"), "{}").unwrap();
        fs::write(p.join("auth/login.json"), r#"{"title":"Login","method":"POST"}"#).unwrap();
        fs::write(p.join("auth/env.json"), r#"{"title":"Nested env is a scenario"}"#).unwrap();
        fs::write(p.join("broken.json"), "nope").unwrap();
        fs::write(p.join("notes.txt"), "").unwrap();
        let entries = list(p).unwrap();
        let rels: Vec<_> = entries.iter().map(|e| e.rel_path.as_str()).collect();
        assert_eq!(rels, ["auth/env.json", "auth/login.json", "broken.json"]);
        assert_eq!(entries[1].method, "POST");
        assert_eq!(entries[2].title, "broken");
        assert_eq!(entries[2].method, UNPARSEABLE_METHOD);
    }

    #[test]
    fn list_rejects_missing_folder() {
        assert!(matches!(list(Path::new("/definitely/not/here")), Err(Error::NotAFolder(_))));
    }
}
