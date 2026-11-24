//! Tells the UI when files in the open project change on disk.

use std::path::Path;
use std::time::Duration;

use cactus_core::layout::WORKSPACE_DIR;
use cactus_core::scenario;
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::events::FsChanged;

const WATCH_DEBOUNCE: Duration = Duration::from_millis(150);
const TEMP_FILE_SUFFIX: &str = ".tmp";

/// Stops watching when dropped.
pub struct ProjectWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
}

impl ProjectWatcher {
    /// Emits [`FsChanged`] for changed files. Returns `None` if the OS
    /// watcher can't start; the app then works without live reload.
    pub fn start(app: &AppHandle, project: &Path) -> Option<Self> {
        let app = app.clone();
        let root = project.to_path_buf();
        let mut debouncer = new_debouncer(WATCH_DEBOUNCE, move |result: DebounceEventResult| {
            let Ok(events) = result else { return };
            let changed = changed_relative_paths(&root, events.iter().map(|e| e.path.as_path()));
            if !changed.is_empty() {
                let _ = FsChanged(changed).emit(&app);
            }
        })
        .ok()?;
        // Watch only the folders scenarios are listed from, so ignored trees
        // like node_modules cost nothing.
        for dir in scenario::walk(project).filter(|e| e.file_type().is_dir()) {
            let _ = debouncer.watcher().watch(dir.path(), RecursiveMode::NonRecursive);
        }
        Some(Self { _debouncer: debouncer })
    }
}

/// Sorted, deduplicated project-relative paths, minus our own writes.
fn changed_relative_paths<'a>(root: &Path, paths: impl Iterator<Item = &'a Path>) -> Vec<String> {
    let mut relative: Vec<String> = paths
        .filter_map(|path| path.strip_prefix(root).ok())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .filter(|path| !is_own_write(path))
        .collect();
    relative.sort();
    relative.dedup();
    relative
}

/// `.cactus/` state and atomic-write temp files are written by the app itself.
fn is_own_write(relative: &str) -> bool {
    relative.starts_with(WORKSPACE_DIR) || relative.ends_with(TEMP_FILE_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_and_dedupes() {
        let root = Path::new("/p");
        let paths =
            ["/p/b.json", "/p/a.json", "/p/b.json", "/p/.cactus/history/x", "/p/a.json.tmp", "/elsewhere/c.json"];
        let changed = changed_relative_paths(root, paths.iter().map(Path::new));
        assert_eq!(changed, ["a.json", "b.json"]);
    }
}
