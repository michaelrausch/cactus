//! Discovering scenario files in a project folder.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

use super::model::default_method;
use crate::collection::model::lenient_string;
use crate::collection::{self, Folder};
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

/// The fields of a request file the listing reads.
#[derive(Deserialize, Default)]
pub(crate) struct ScenarioHeader {
    #[serde(default)]
    info: HeaderInfo,
    #[serde(default)]
    http: HeaderHttp,
}

#[derive(Deserialize, Default)]
struct HeaderInfo {
    #[serde(default, deserialize_with = "lenient_string")]
    name: String,
    #[serde(default)]
    seq: Option<f64>,
}

#[derive(Deserialize)]
struct HeaderHttp {
    #[serde(default = "default_method", deserialize_with = "lenient_string")]
    method: String,
}

impl Default for HeaderHttp {
    fn default() -> Self {
        Self { method: default_method() }
    }
}

impl ScenarioHeader {
    /// Unreadable or unparseable files read as untitled with a `???` method.
    pub(crate) fn read(path: &Path) -> Self {
        let parsed = fs::read_to_string(path).ok().and_then(|text| serde_yaml_ng::from_str::<Self>(&text).ok());
        parsed.unwrap_or_else(|| Self { http: HeaderHttp { method: UNPARSEABLE_METHOD.into() }, ..Self::default() })
    }

    fn seq(&self) -> Option<f64> {
        self.info.seq
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

/// Every request in the project in tree order, as Bruno lists and runs them:
/// at each level, folders and requests sorted together by `seq`, then by
/// name. Unparseable files are included so the user can see and fix them.
pub fn list(project: &Path) -> Result<Vec<ScenarioEntry>> {
    if !project.is_dir() {
        return Err(Error::NotAFolder(project.to_path_buf()));
    }
    let mut folder_seqs = HashMap::new();
    let mut found: Vec<(Vec<(f64, String)>, ScenarioEntry)> = walk(project)
        .filter_map(|e| read_entry(project, &e))
        .map(|(entry, seq)| (tree_position(project, &entry.rel_path, seq, &mut folder_seqs), entry))
        .collect();
    found.sort_by(|(a, _), (b, _)| {
        let steps =
            a.iter().zip(b).map(|((a_seq, a_name), (b_seq, b_name))| a_seq.total_cmp(b_seq).then(a_name.cmp(b_name)));
        steps.fold(Ordering::Equal, Ordering::then).then(a.len().cmp(&b.len()))
    });
    Ok(found.into_iter().map(|(_, entry)| entry).collect())
}

/// `(seq, name)` of each folder on the way to the request, then of the
/// request. Anything without a `seq` sorts after its numbered siblings.
fn tree_position(
    project: &Path,
    path: &ScenarioPath,
    seq: Option<f64>,
    folder_seqs: &mut HashMap<String, Option<f64>>,
) -> Vec<(f64, String)> {
    let unnumbered = |seq: Option<f64>| seq.unwrap_or(f64::INFINITY);
    let mut position = Vec::new();
    let mut dir = String::new();
    for name in path.dir().split('/').filter(|part| !part.is_empty()) {
        dir = if dir.is_empty() { name.to_string() } else { format!("{dir}/{name}") };
        let folder_seq = *folder_seqs.entry(dir.clone()).or_insert_with(|| folder_seq(project, &dir));
        position.push((unnumbered(folder_seq), name.to_string()));
    }
    position.push((unnumbered(seq), path.file_name().to_string()));
    position
}

fn folder_seq(project: &Path, dir: &str) -> Option<f64> {
    collection::read_if_exists::<Folder>(&layout::folder_file(project, dir)).ok().flatten()?.info.seq
}

/// One past the highest `seq` among the requests and folders directly in
/// `dir`, which share one order.
pub(crate) fn next_seq(project: &Path, dir: &str) -> f64 {
    let highest = fs::read_dir(project.join(dir))
        .into_iter()
        .flatten()
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = ScenarioPath::in_dir(dir, &name).ok()?;
            if entry.path().is_dir() {
                folder_seq(project, rel.as_str())
            } else {
                layout::is_request_file(rel.as_str()).then(|| ScenarioHeader::read(&entry.path()).seq()).flatten()
            }
        })
        .fold(0.0, f64::max);
    highest + 1.0
}

fn read_entry(project: &Path, entry: &DirEntry) -> Option<(ScenarioEntry, Option<f64>)> {
    let too_big = entry.metadata().is_ok_and(|m| m.len() > MAX_SCENARIO_BYTES);
    if !entry.file_type().is_file() || too_big {
        return None;
    }
    let rel_path = ScenarioPath::from_absolute(project, entry.path())?;
    if !layout::is_request_file(rel_path.as_str()) {
        return None;
    }
    let header = ScenarioHeader::read(entry.path());
    let title = if header.info.name.is_empty() { fallback_title(&rel_path) } else { header.info.name };
    Some((ScenarioEntry { rel_path, title, method: header.http.method }, header.info.seq))
}

fn fallback_title(path: &ScenarioPath) -> String {
    path.as_str().trim_end_matches(layout::REQUEST_EXTENSION).to_string()
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
        fs::create_dir_all(p.join("environments")).unwrap();
        fs::write(p.join("node_modules/pkg/x.yml"), "").unwrap();
        fs::write(p.join("opencollection.yml"), "opencollection: 1.0.0").unwrap();
        fs::write(p.join("environments/dev.yml"), "name: dev").unwrap();
        fs::write(p.join(".cactus/x.yml"), "").unwrap();
        fs::write(p.join("auth/folder.yml"), "info: {name: Auth, type: folder, seq: 2}").unwrap();
        fs::write(p.join("first.yml"), "info: {name: First, seq: 1}").unwrap();
        fs::write(p.join("auth/login.yml"), "info: {name: Login, seq: 2}\nhttp: {method: POST}").unwrap();
        fs::write(p.join("auth/me.yml"), "info: {name: Me, seq: 1}").unwrap();
        fs::write(p.join("broken.yml"), "info: [nope").unwrap();
        fs::write(p.join("notes.json"), "{}").unwrap();
        let entries = list(p).unwrap();
        let rels: Vec<_> = entries.iter().map(|e| e.rel_path.as_str()).collect();
        assert_eq!(rels, ["first.yml", "auth/me.yml", "auth/login.yml", "broken.yml"]);
        assert_eq!(entries[2].method, "POST");
        assert_eq!(entries[1].method, "GET");
        assert_eq!(entries[3].title, "broken");
        assert_eq!(entries[3].method, UNPARSEABLE_METHOD);
        assert_eq!(next_seq(p, ""), 3.0);
        assert_eq!(next_seq(p, "auth"), 3.0);
    }

    #[test]
    fn list_rejects_missing_folder() {
        assert!(matches!(list(Path::new("/definitely/not/here")), Err(Error::NotAFolder(_))));
    }
}
