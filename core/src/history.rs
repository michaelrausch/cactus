//! Recent responses per request, in `.cactus/history/`.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::best_effort;
use crate::http::Response;
use crate::storage::{ensure_parent_dir, read_json_lenient, write_json};
use crate::text::truncate_utf8;
use crate::{ProjectPath, Result, layout};

const MAX_HISTORY_ENTRIES: usize = 10;
/// Older entries are trimmed harder than live bodies to keep files small.
/// The newest entry is never trimmed (see [`push`]).
const MAX_HISTORY_BODY_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct HistoryEntry {
    /// Unix epoch millis.
    // A JS `number`, accepting precision loss: epoch millis stay far below 2^53.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub at: u64,
    pub response: Response,
}

/// Newest first. A missing or corrupt history is simply empty.
pub fn load(project: &Path, scenario: &ProjectPath) -> Vec<HistoryEntry> {
    read_json_lenient(&layout::history_file(project, scenario))
}

/// Prepend `entry`, keeping at most [`MAX_HISTORY_ENTRIES`]. The newest entry
/// keeps its full body, since reopening a scenario shows it and a cut-off JSON
/// body can't be parsed; only older entries are trimmed.
pub fn push(project: &Path, scenario: &ProjectPath, entry: HistoryEntry) -> Result<()> {
    layout::ensure_workspace(project)?;
    let file = layout::history_file(project, scenario);
    ensure_parent_dir(&file)?;
    let mut entries = load(project, scenario);
    for older in &mut entries {
        trim_body(&mut older.response);
    }
    entries.insert(0, entry);
    entries.truncate(MAX_HISTORY_ENTRIES);
    write_json(&file, &entries)
}

/// Best effort: a scenario without history is not an error.
pub fn clear(project: &Path, scenario: &ProjectPath) {
    ignore_missing(fs::remove_file(layout::history_file(project, scenario)), "clear history");
}

/// Best effort, like [`clear`].
pub fn rename(project: &Path, from: &ProjectPath, to: &ProjectPath) {
    ignore_missing(fs::rename(layout::history_file(project, from), layout::history_file(project, to)), "move history");
}

/// No history yet is normal; any other failure is logged.
fn ignore_missing(result: std::io::Result<()>, what: &str) {
    if let Err(error) = &result
        && error.kind() == std::io::ErrorKind::NotFound
    {
        return;
    }
    best_effort::log(what, result);
}

fn trim_body(response: &mut Response) {
    if truncate_utf8(&mut response.body, MAX_HISTORY_BODY_BYTES) {
        response.truncated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(at: u64, body: &str) -> HistoryEntry {
        HistoryEntry {
            at,
            response: Response {
                method: "GET".into(),
                url: String::new(),
                status: 200,
                status_text: String::new(),
                headers: Vec::new(),
                body: body.into(),
                truncated: false,
                time_ms: 0,
                size_bytes: body.len(),
                unresolved: Vec::new(),
                warnings: Vec::new(),
                captured: std::collections::BTreeMap::default(),
                capture_failures: vec![],
                checks: Vec::new(),
                critter: None,
                relogged: false,
            },
        }
    }

    #[test]
    fn history_from_before_critters_loads() {
        let old = r#"[{"at":1,"response":{"method":"GET","url":"","status":200,"statusText":"OK","headers":[],
            "body":"","truncated":false,"timeMs":1,"sizeBytes":0,"unresolved":[]}}]"#;
        let entries: Vec<HistoryEntry> = serde_json::from_str(old).unwrap();
        assert_eq!((entries[0].response.critter.as_deref(), entries[0].response.relogged), (None, false));
    }

    #[test]
    fn newest_entry_keeps_full_body() {
        let dir = tempfile::tempdir().unwrap();
        let path = ProjectPath::parse("a.yml").unwrap();
        let big = "x".repeat(MAX_HISTORY_BODY_BYTES + 10);

        push(dir.path(), &path, entry(1, &big)).unwrap();
        let entries = load(dir.path(), &path);
        assert_eq!(entries[0].response.body.len(), big.len());
        assert!(!entries[0].response.truncated);

        push(dir.path(), &path, entry(2, "small")).unwrap();
        let entries = load(dir.path(), &path);
        assert_eq!(entries[0].response.body, "small");
        assert_eq!(entries[1].response.body.len(), MAX_HISTORY_BODY_BYTES);
        assert!(entries[1].response.truncated);
    }
}
