//! Tells the UI when files in the open project change on disk.

use std::path::Path;
use std::time::Duration;

use cactus_core::layout::TEMP_FILE_SUFFIX;
use cactus_core::scenario;
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::events::FsChanged;

const WATCH_DEBOUNCE: Duration = Duration::from_millis(150);

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
        watch_project(debouncer.watcher(), project);
        Some(Self { _debouncer: debouncer })
    }
}

/// Windows watches a tree recursively through a single handle, which is
/// cheap. A watch per folder would hold a handle on each, and Windows then
/// refuses to rename or delete their parents in Explorer.
#[cfg(windows)]
fn watch_project(watcher: &mut dyn Watcher, project: &Path) {
    let _ = watcher.watch(project, RecursiveMode::Recursive);
}

/// Elsewhere a recursive watch costs one watch per folder, so only the
/// folders requests are listed from are watched, and ignored trees like
/// `node_modules` cost nothing.
#[cfg(not(windows))]
fn watch_project(watcher: &mut dyn Watcher, project: &Path) {
    for dir in scenario::walk(project).filter(|e| e.file_type().is_dir()) {
        let _ = watcher.watch(dir.path(), RecursiveMode::NonRecursive);
    }
}

/// Sorted, deduplicated project-relative paths, minus atomic-write temp files
/// and anything the listing leaves out (`.cactus/` included, as a dot-folder).
fn changed_relative_paths<'a>(root: &Path, paths: impl Iterator<Item = &'a Path>) -> Vec<String> {
    let mut relative: Vec<String> = paths
        .filter_map(|path| path.strip_prefix(root).ok())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .filter(|path| !path.ends_with(TEMP_FILE_SUFFIX) && !scenario::is_ignored(path))
        .collect();
    relative.sort();
    relative.dedup();
    relative
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_and_dedupes() {
        let root = Path::new("/p");
        let paths = [
            "/p/b.json",
            "/p/a.json",
            "/p/b.json",
            "/p/.cactus/history/x",
            "/p/a.json.tmp",
            "/elsewhere/c.json",
            "/p/web/node_modules/x/y.yml",
            "/p/.git/index",
        ];
        let changed = changed_relative_paths(root, paths.iter().map(Path::new));
        assert_eq!(changed, ["a.json", "b.json"]);
    }
}
