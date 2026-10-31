//! The app-wide list of recently opened projects.

use std::path::Path;

use crate::Result;
use crate::storage::{ensure_parent_dir, read_json_lenient, write_json};

const MAX_RECENT_PROJECTS: usize = 15;

pub fn load(file: &Path) -> Vec<String> {
    read_json_lenient(file)
}

/// Move `project` to the front of the list and return the new list. An
/// earlier entry for the same folder, spelt differently, is replaced.
pub fn push(file: &Path, project: &str) -> Result<Vec<String>> {
    let mut projects = load(file);
    let key = folder_key(project);
    projects.retain(|p| folder_key(p) != key);
    projects.insert(0, project.to_string());
    projects.truncate(MAX_RECENT_PROJECTS);
    ensure_parent_dir(file)?;
    write_json(file, &projects)?;
    Ok(projects)
}

/// What two spellings of one folder have in common: no trailing separator
/// (except at a root like `/` or `C:\`) and, on Windows, where paths are
/// case-insensitive and either separator works, one case and one separator.
fn folder_key(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    let is_root = trimmed.is_empty() || trimmed.ends_with(':');
    let path = if is_root { path } else { trimmed };
    if cfg!(windows) { path.replace('/', "\\").to_lowercase() } else { path.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_dedupes_and_orders() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("cfg/recent.json");
        push(&f, "/a").unwrap();
        push(&f, "/b").unwrap();
        assert_eq!(push(&f, "/a").unwrap(), ["/a", "/b"]);
        assert_eq!(push(&f, "/b/").unwrap(), ["/b/", "/a"]);
        assert_eq!(push(&f, "/").unwrap(), ["/", "/b/", "/a"]);
    }

    #[test]
    #[cfg(windows)]
    fn windows_spellings_of_one_folder_are_one_entry() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("recent.json");
        push(&f, r"C:\Users\me\api").unwrap();
        assert_eq!(push(&f, "c:/users/ME/api/").unwrap(), ["c:/users/ME/api/"]);
        assert_eq!(folder_key(r"C:\"), folder_key("c:/"));
    }
}
