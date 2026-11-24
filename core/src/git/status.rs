//! What changed in the project since the last commit.

use std::path::Path;

use git2::{DiffDelta, Status, StatusEntry, StatusOptions};
use serde::Serialize;

use super::commit::CommitSummary;
use super::repo::ProjectRepo;
use crate::{ProjectPath, Result};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GitStatus {
    /// `None` when HEAD is detached.
    pub branch: Option<String>,
    /// Changed files in the project, sorted by path.
    pub files: Vec<FileStatus>,
    /// The operation in progress (`"merge"`, `"rebase"`, …) that blocks committing.
    pub busy: Option<String>,
    /// Files staged elsewhere in the repository, which a commit includes too.
    pub staged_outside: u32,
    /// The commit HEAD points at; `None` before the first one.
    pub last_commit: Option<CommitSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FileStatus {
    pub path: ProjectPath,
    /// The previous path of a staged rename, if it was in the project.
    pub renamed_from: Option<ProjectPath>,
    /// The change in the index, which the next commit records.
    pub staged: Option<ChangeKind>,
    /// The change in the work tree that isn't staged.
    pub unstaged: Option<ChangeKind>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
}

/// The project's changes, or `None` if it isn't in a git repository.
pub fn status(project: &Path) -> Result<Option<GitStatus>> {
    let Some(repo) = ProjectRepo::open(project)? else { return Ok(None) };
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true)
        .disable_pathspec_match(true);
    for spec in repo.pathspec() {
        options.pathspec(spec);
    }
    let statuses = repo.repo.statuses(Some(&mut options))?;
    let mut files: Vec<FileStatus> = statuses.iter().filter_map(|entry| file_status(&repo, &entry)).collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Some(GitStatus {
        branch: repo.branch(),
        files,
        busy: repo.busy().map(str::to_string),
        staged_outside: staged_outside(&repo)?,
        last_commit: repo.head_commit()?.as_ref().map(CommitSummary::of).transpose()?,
    }))
}

fn file_status(repo: &ProjectRepo, entry: &StatusEntry<'_>) -> Option<FileStatus> {
    let status = entry.status();
    let (staged, unstaged) = if status.is_conflicted() {
        (None, Some(ChangeKind::Conflicted))
    } else {
        (staged_kind(status), unstaged_kind(status))
    };
    if staged.is_none() && unstaged.is_none() {
        return None;
    }
    // A staged rename's entry is named after its old path; list it under the new one.
    let delta = entry.head_to_index().or_else(|| entry.index_to_workdir());
    let path = delta.as_ref().and_then(new_path).or_else(|| entry.path().map(str::to_string))?;
    let renamed_from = entry
        .head_to_index()
        .filter(|_| status.is_index_renamed())
        .and_then(|delta| delta.old_file().path().and_then(Path::to_str).and_then(|old| repo.project_path(old)));
    Some(FileStatus { path: repo.project_path(&path)?, renamed_from, staged, unstaged })
}

fn new_path(delta: &DiffDelta<'_>) -> Option<String> {
    delta.new_file().path().and_then(Path::to_str).map(str::to_string)
}

fn staged_kind(status: Status) -> Option<ChangeKind> {
    if status.is_index_new() {
        Some(ChangeKind::Added)
    } else if status.is_index_deleted() {
        Some(ChangeKind::Deleted)
    } else if status.is_index_renamed() {
        Some(ChangeKind::Renamed)
    } else if status.is_index_modified() || status.is_index_typechange() {
        Some(ChangeKind::Modified)
    } else {
        None
    }
}

fn unstaged_kind(status: Status) -> Option<ChangeKind> {
    if status.is_wt_new() {
        Some(ChangeKind::Untracked)
    } else if status.is_wt_deleted() {
        Some(ChangeKind::Deleted)
    } else if status.is_wt_renamed() {
        Some(ChangeKind::Renamed)
    } else if status.is_wt_modified() || status.is_wt_typechange() {
        Some(ChangeKind::Modified)
    } else {
        None
    }
}

/// Staged changes outside the project folder; always 0 at the work-tree root.
fn staged_outside(repo: &ProjectRepo) -> Result<u32> {
    if repo.pathspec().is_empty() {
        return Ok(0);
    }
    let tree = repo.head_tree()?;
    let diff = repo.repo.diff_tree_to_index(tree.as_ref(), None, None)?;
    let outside = diff
        .deltas()
        .filter(|delta| {
            new_path(delta)
                .or_else(|| delta.old_file().path()?.to_str().map(str::to_string))
                .is_some_and(|p| repo.project_path(&p).is_none())
        })
        .count();
    Ok(u32::try_from(outside).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::testing::{commit_all, init, stage_everything, write};
    use super::*;

    fn kinds(status: &GitStatus) -> Vec<(&str, Option<ChangeKind>, Option<ChangeKind>)> {
        status.files.iter().map(|f| (f.path.as_str(), f.staged, f.unstaged)).collect()
    }

    #[test]
    fn untracked_modified_and_deleted() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "a.yml", "a");
        write(p, "b.yml", "b");
        write(p, ".cactus/.gitignore", "*\n");
        write(p, ".cactus/state.json", "{}");
        commit_all(&repo, "first");
        write(p, "a.yml", "changed");
        fs::remove_file(p.join("b.yml")).unwrap();
        write(p, "auth/login.yml", "new");

        let status = status(p).unwrap().unwrap();
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.busy, None);
        assert_eq!(
            kinds(&status),
            [
                ("a.yml", None, Some(ChangeKind::Modified)),
                ("auth/login.yml", None, Some(ChangeKind::Untracked)),
                ("b.yml", None, Some(ChangeKind::Deleted)),
            ]
        );
    }

    #[test]
    fn only_lists_the_project_in_a_subfolder() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "README.md", "x");
        write(p, "api/a.yml", "a");
        write(p, "api-other/b.yml", "b");
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();

        let status = status(&p.join("api")).unwrap().unwrap();
        assert_eq!(kinds(&status), [("a.yml", None, Some(ChangeKind::Untracked))]);
        assert_eq!(status.staged_outside, 1);
    }

    #[test]
    fn staged_rename_is_listed_under_its_new_path() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "old.yml", "the same content, long enough to be found similar\n");
        commit_all(&repo, "first");
        fs::rename(p.join("old.yml"), p.join("new.yml")).unwrap();
        stage_everything(&repo);

        let status = status(p).unwrap().unwrap();
        assert_eq!(kinds(&status), [("new.yml", Some(ChangeKind::Renamed), None)]);
        assert_eq!(status.files[0].renamed_from.as_ref().map(ProjectPath::as_str), Some("old.yml"));
    }
}
