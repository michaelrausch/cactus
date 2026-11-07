//! Recording the index as a commit on the checked-out branch.

use std::path::Path;

use git2::{Commit, ErrorCode};
use serde::Serialize;

use super::repo::ProjectRepo;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CommitSummary {
    /// The abbreviated commit id.
    pub id: String,
    /// The message's first line.
    pub summary: String,
    /// When it was committed, in Unix epoch millis.
    // Exposed to TypeScript as `number`: epoch millis stay far below 2^53.
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub at: u64,
}

impl CommitSummary {
    pub(super) fn of(commit: &Commit<'_>) -> Result<Self> {
        let short_id = commit.as_object().short_id()?;
        let seconds = u64::try_from(commit.time().seconds()).unwrap_or_default();
        Ok(Self {
            id: short_id.as_str().unwrap_or_default().to_string(),
            summary: commit.summary().unwrap_or_default().to_string(),
            at: seconds * 1000,
        })
    }
}

/// Commit everything staged in the repository, as `git commit` does; that
/// can include files outside the project (see `GitStatus::staged_outside`).
/// The author is `user.name`/`user.email` from git config. Hooks don't run.
pub fn commit(project: &Path, message: &str) -> Result<CommitSummary> {
    let message = message.trim();
    if message.is_empty() {
        return Err(Error::EmptyCommitMessage);
    }
    let repo = ProjectRepo::require(project)?;
    if let Some(operation) = repo.busy() {
        return Err(Error::RepositoryBusy(operation.to_string()));
    }
    let tree = repo.repo.find_tree(repo.repo.index()?.write_tree()?)?;
    let parent = repo.head_commit()?;
    let unchanged = match &parent {
        Some(parent) => parent.tree_id() == tree.id(),
        None => tree.is_empty(),
    };
    if unchanged {
        return Err(Error::NothingToCommit);
    }
    let signature = repo.repo.signature().map_err(|e| match e.code() {
        ErrorCode::NotFound => Error::GitIdentityMissing,
        _ => e.into(),
    })?;
    let parents: Vec<_> = parent.iter().collect();
    let id = repo.repo.commit(Some("HEAD"), &signature, &signature, &format!("{message}\n"), &tree, &parents)?;
    CommitSummary::of(&repo.repo.find_commit(id)?)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::stage::stage_all;
    use super::super::status::status;
    use super::super::testing::{commit_all, init, write};
    use super::*;

    #[test]
    fn first_and_second_commit() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "a.yml", "a");
        stage_all(p).unwrap();
        let first = commit(p, "  Add a\n\nWith a body.\n").unwrap();
        assert_eq!(first.summary, "Add a");
        assert_eq!(status(p).unwrap().unwrap().files, []);

        write(p, "a.yml", "changed");
        stage_all(p).unwrap();
        let second = commit(p, "Change a").unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert!(head.id().to_string().starts_with(&second.id));
        assert_eq!(head.message(), Some("Change a\n"));
        assert_eq!(head.author().email(), Some("test@example.com"));
        assert_eq!(head.parent(0).unwrap().message(), Some("Add a\n\nWith a body.\n"));
        assert_eq!(status(p).unwrap().unwrap().last_commit, Some(second));
    }

    #[test]
    fn refuses_an_empty_message_or_nothing_staged() {
        let (dir, repo) = init();
        let p = dir.path();
        assert!(matches!(commit(p, " \n"), Err(Error::EmptyCommitMessage)));
        assert!(matches!(commit(p, "Nothing yet"), Err(Error::NothingToCommit)));
        write(p, "a.yml", "a");
        commit_all(&repo, "first");
        write(p, "a.yml", "unstaged");
        assert!(matches!(commit(p, "Still nothing"), Err(Error::NothingToCommit)));
    }

    #[test]
    fn refuses_during_a_merge() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "a.yml", "a");
        commit_all(&repo, "first");
        let head = repo.head().unwrap().target().unwrap();
        fs::write(repo.path().join("MERGE_HEAD"), format!("{head}\n")).unwrap();
        write(p, "a.yml", "b");
        stage_all(p).unwrap();
        assert!(matches!(commit(p, "Merge"), Err(Error::RepositoryBusy(op)) if op == "merge"));
        assert_eq!(status(p).unwrap().unwrap().busy.as_deref(), Some("merge"));
    }
}
