//! A file's changes, line by line.

use std::path::Path;

use git2::{Diff, DiffOptions, Patch};
use serde::{Deserialize, Serialize};

use super::repo::ProjectRepo;
use crate::{Result, ScenarioPath};

/// Enough to review any request file; anything longer is cut off.
const MAX_DIFF_LINES: usize = 2000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum DiffSide {
    /// HEAD → index: what the next commit records.
    Staged,
    /// Index → work tree: what isn't staged yet.
    Unstaged,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FileDiff {
    /// Binary files have no hunks.
    pub binary: bool,
    /// More than [`MAX_DIFF_LINES`] lines changed, and the rest are left out.
    pub truncated: bool,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DiffHunk {
    /// `@@ -1,3 +1,4 @@`
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    /// The line without its line ending.
    pub text: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

/// One side of `path`'s changes. Untracked files show as wholly added.
pub fn diff(project: &Path, path: &ScenarioPath, side: DiffSide) -> Result<FileDiff> {
    let repo = ProjectRepo::require(project)?;
    let mut options = DiffOptions::new();
    options
        .pathspec(repo.repo_path(path))
        .disable_pathspec_match(true)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    let diff = match side {
        DiffSide::Staged => repo.repo.diff_tree_to_index(repo.head_tree()?.as_ref(), None, Some(&mut options))?,
        DiffSide::Unstaged => repo.repo.diff_index_to_workdir(None, Some(&mut options))?,
    };
    collect(&diff)
}

fn collect(diff: &Diff<'_>) -> Result<FileDiff> {
    let mut file_diff = FileDiff::default();
    let mut line_count = 0;
    for delta in 0..diff.deltas().len() {
        let Some(patch) = Patch::from_diff(diff, delta)? else { continue };
        if patch.delta().flags().is_binary() {
            file_diff.binary = true;
            continue;
        }
        for hunk_index in 0..patch.num_hunks() {
            let (hunk, hunk_lines) = patch.hunk(hunk_index)?;
            let mut lines = Vec::with_capacity(hunk_lines);
            for line_index in 0..hunk_lines {
                if line_count == MAX_DIFF_LINES {
                    file_diff.truncated = true;
                    break;
                }
                let line = patch.line_in_hunk(hunk_index, line_index)?;
                let kind = match line.origin() {
                    ' ' => DiffLineKind::Context,
                    '+' => DiffLineKind::Added,
                    '-' => DiffLineKind::Removed,
                    // End-of-file newline markers.
                    _ => continue,
                };
                let text = String::from_utf8_lossy(line.content()).trim_end_matches(['\n', '\r']).to_string();
                lines.push(DiffLine { kind, old_line: line.old_lineno(), new_line: line.new_lineno(), text });
                line_count += 1;
            }
            let header = String::from_utf8_lossy(hunk.header()).trim_end().to_string();
            file_diff.hunks.push(DiffHunk { header, lines });
            if file_diff.truncated {
                return Ok(file_diff);
            }
        }
    }
    Ok(file_diff)
}

#[cfg(test)]
mod tests {
    use super::super::stage::stage;
    use super::super::testing::{commit_all, init, write};
    use super::*;

    fn path(rel: &str) -> ScenarioPath {
        ScenarioPath::parse(rel).unwrap()
    }

    fn lines(diff: &FileDiff) -> Vec<(DiffLineKind, &str)> {
        diff.hunks.iter().flat_map(|h| &h.lines).map(|l| (l.kind, l.text.as_str())).collect()
    }

    #[test]
    fn staged_and_unstaged_sides() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "a.yml", "one\ntwo\n");
        commit_all(&repo, "first");
        write(p, "a.yml", "one\n2\n");
        stage(p, &[path("a.yml")]).unwrap();
        write(p, "a.yml", "one\n2\nthree\n");

        let staged = diff(p, &path("a.yml"), DiffSide::Staged).unwrap();
        assert_eq!(
            lines(&staged),
            [(DiffLineKind::Context, "one"), (DiffLineKind::Removed, "two"), (DiffLineKind::Added, "2")]
        );
        assert_eq!(staged.hunks[0].header, "@@ -1,2 +1,2 @@");
        let unstaged = diff(p, &path("a.yml"), DiffSide::Unstaged).unwrap();
        assert_eq!(lines(&unstaged).last(), Some(&(DiffLineKind::Added, "three")));
        assert_eq!(unstaged.hunks[0].lines[2].new_line, Some(3));
    }

    #[test]
    fn untracked_file_in_a_subfolder_project() {
        let (dir, _repo) = init();
        write(dir.path(), "api/new.yml", "a\nb\n");
        let diff = diff(&dir.path().join("api"), &path("new.yml"), DiffSide::Unstaged).unwrap();
        assert_eq!(lines(&diff), [(DiffLineKind::Added, "a"), (DiffLineKind::Added, "b")]);
    }

    #[test]
    fn binary_and_truncated() {
        let (dir, _repo) = init();
        let p = dir.path();
        std::fs::write(p.join("logo.png"), [0u8, 159, 146, 150, 0, 1]).unwrap();
        let long = (0..=MAX_DIFF_LINES).map(|n| n.to_string()).collect::<Vec<_>>().join("\n");
        write(p, "long.yml", &long);

        let binary = diff(p, &path("logo.png"), DiffSide::Unstaged).unwrap();
        assert!(binary.binary && binary.hunks.is_empty());
        let truncated = diff(p, &path("long.yml"), DiffSide::Unstaged).unwrap();
        assert!(truncated.truncated);
        assert_eq!(lines(&truncated).len(), MAX_DIFF_LINES);
    }
}
