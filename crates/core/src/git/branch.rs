//! Local branches: listing, creating, switching and deleting them.

use std::path::Path;

use git2::build::CheckoutBuilder;
use git2::{Branch, BranchType, ErrorCode};
use serde::Serialize;

use super::commit::CommitSummary;
use super::repo::ProjectRepo;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BranchInfo {
    pub name: String,
    /// Whether it is checked out.
    pub current: bool,
    /// Whether HEAD contains all its commits, so deleting it loses nothing.
    pub merged: bool,
    pub last_commit: Option<CommitSummary>,
}

/// Every local branch, by name. Before the first commit there are none.
pub fn branches(project: &Path) -> Result<Vec<BranchInfo>> {
    let repo = ProjectRepo::require(project)?;
    let head = repo.head_commit()?.map(|commit| commit.id());
    let current = repo.branch();
    let mut infos = Vec::new();
    for entry in repo.repo.branches(Some(BranchType::Local))? {
        let (branch, _) = entry?;
        let Some(name) = branch.name()?.map(str::to_string) else { continue };
        let commit = branch.get().peel_to_commit()?;
        let merged = match head {
            Some(head) => head == commit.id() || repo.repo.graph_descendant_of(head, commit.id())?,
            None => false,
        };
        infos.push(BranchInfo {
            current: current.as_deref() == Some(name.as_str()),
            merged,
            last_commit: Some(CommitSummary::of(&commit)?),
            name,
        });
    }
    infos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(infos)
}

/// Branch off the current commit, and check the new branch out if `switch`.
/// Uncommitted changes stay as they are either way.
pub fn create_branch(project: &Path, name: &str, switch: bool) -> Result<()> {
    let repo = ProjectRepo::require(project)?;
    validate_name(name)?;
    let head = repo.head_commit()?.ok_or(Error::NoCommitsYet)?;
    if repo.repo.find_branch(name, BranchType::Local).is_ok() {
        return Err(Error::AlreadyExists(format!("branch {name}")));
    }
    let branch = repo.repo.branch(name, &head, false)?;
    if switch {
        // Same commit, so the work tree and index already match it.
        repo.repo.set_head(reference_name(&branch)?)?;
    }
    Ok(())
}

/// Check out branch `name`, as `git switch` does: uncommitted changes come
/// along, unless the switch would overwrite them, in which case nothing changes.
pub fn switch_branch(project: &Path, name: &str) -> Result<()> {
    let repo = ProjectRepo::require(project)?;
    if let Some(operation) = repo.busy() {
        return Err(Error::RepositoryBusy(operation.to_string()));
    }
    let branch = find(&repo, name)?;
    let target = branch.get().peel_to_commit()?;
    let mut checkout = CheckoutBuilder::new();
    checkout.safe();
    repo.repo.checkout_tree(target.as_object(), Some(&mut checkout)).map_err(|e| match e.code() {
        ErrorCode::Conflict => Error::CheckoutConflict(name.to_string()),
        _ => e.into(),
    })?;
    repo.repo.set_head(reference_name(&branch)?)?;
    Ok(())
}

/// Delete branch `name`. One with commits HEAD doesn't contain is kept
/// unless `force`, since they would be lost.
pub fn delete_branch(project: &Path, name: &str, force: bool) -> Result<()> {
    let repo = ProjectRepo::require(project)?;
    let info = branches(project)?.into_iter().find(|b| b.name == name);
    if info.as_ref().is_some_and(|b| b.current) {
        return Err(Error::DeleteCurrentBranch(name.to_string()));
    }
    if !force && info.is_some_and(|b| !b.merged) {
        return Err(Error::BranchNotMerged(name.to_string()));
    }
    find(&repo, name)?.delete()?;
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if Branch::name_is_valid(name)? { Ok(()) } else { Err(Error::InvalidBranchName(name.to_string())) }
}

fn find<'r>(repo: &'r ProjectRepo, name: &str) -> Result<Branch<'r>> {
    Ok(repo.repo.find_branch(name, BranchType::Local)?)
}

fn reference_name<'b>(branch: &'b Branch<'_>) -> Result<&'b str> {
    branch
        .get()
        .name()
        .ok_or_else(|| Error::InvalidBranchName(String::from_utf8_lossy(branch.get().name_bytes()).into()))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::testing::{commit_all, init, write};
    use super::*;

    fn names(project: &Path) -> Vec<(String, bool, bool)> {
        branches(project).unwrap().into_iter().map(|b| (b.name, b.current, b.merged)).collect()
    }

    #[test]
    fn needs_a_first_commit() {
        let (dir, _repo) = init();
        assert_eq!(names(dir.path()), []);
        assert!(matches!(create_branch(dir.path(), "feature", true), Err(Error::NoCommitsYet)));
    }

    #[test]
    fn create_switch_and_delete() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "a.yml", "main");
        commit_all(&repo, "first");

        create_branch(p, "feature", true).unwrap();
        assert_eq!(names(p), [("feature".into(), true, true), ("main".into(), false, true)]);
        write(p, "a.yml", "feature");
        commit_all(&repo, "on feature");

        switch_branch(p, "main").unwrap();
        assert_eq!(fs::read_to_string(p.join("a.yml")).unwrap(), "main");
        assert_eq!(names(p)[0], ("feature".into(), false, false));
        assert!(matches!(delete_branch(p, "feature", false), Err(Error::BranchNotMerged(_))));
        assert!(matches!(delete_branch(p, "main", true), Err(Error::DeleteCurrentBranch(_))));
        delete_branch(p, "feature", true).unwrap();
        assert_eq!(names(p), [("main".into(), true, true)]);
    }

    #[test]
    fn rejects_bad_and_taken_names() {
        let (dir, repo) = init();
        write(dir.path(), "a.yml", "a");
        commit_all(&repo, "first");
        assert!(matches!(create_branch(dir.path(), "no spaces", false), Err(Error::InvalidBranchName(_))));
        assert!(matches!(create_branch(dir.path(), "main", false), Err(Error::AlreadyExists(_))));
    }

    #[test]
    fn switching_keeps_changes_it_would_not_overwrite() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "a.yml", "main");
        write(p, "b.yml", "b");
        commit_all(&repo, "first");
        create_branch(p, "feature", true).unwrap();
        write(p, "a.yml", "feature");
        commit_all(&repo, "on feature");
        switch_branch(p, "main").unwrap();

        write(p, "b.yml", "edited");
        switch_branch(p, "feature").unwrap();
        assert_eq!(fs::read_to_string(p.join("b.yml")).unwrap(), "edited");

        write(p, "a.yml", "uncommitted");
        assert!(matches!(switch_branch(p, "main"), Err(Error::CheckoutConflict(_))));
        assert_eq!(fs::read_to_string(p.join("a.yml")).unwrap(), "uncommitted");
        assert_eq!(repo.head().unwrap().shorthand(), Some("feature"));
    }
}
