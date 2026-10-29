//! The app-wide list of recently opened projects.

use std::path::Path;

use crate::Result;
use crate::storage::{ensure_parent_dir, read_json_lenient, write_json};

const MAX_RECENT_PROJECTS: usize = 15;

pub fn load(file: &Path) -> Vec<String> {
    read_json_lenient(file)
}

/// Move `project` to the front of the list and return the new list.
pub fn push(file: &Path, project: &str) -> Result<Vec<String>> {
    let mut projects = load(file);
    projects.retain(|p| p != project);
    projects.insert(0, project.to_string());
    projects.truncate(MAX_RECENT_PROJECTS);
    ensure_parent_dir(file)?;
    write_json(file, &projects)?;
    Ok(projects)
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
    }
}
