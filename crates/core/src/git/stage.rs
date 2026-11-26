//! Moving changes into and out of the index, and throwing them away.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use git2::IndexAddOption;
use git2::build::CheckoutBuilder;

use super::repo::ProjectRepo;
use super::status::status;
use crate::{ProjectPath, Result};

/// Stage the files as they are now, deletions included.
pub fn stage(project: &Path, paths: &[ProjectPath]) -> Result<()> {
    let repo = ProjectRepo::require(project)?;
    let mut index = repo.repo.index()?;
    for path in paths {
        let repo_path = repo.repo_path(path);
        if fs::symlink_metadata(path.resolve(project)).is_ok() {
            index.add_path(Path::new(&repo_path))?;
        } else {
            index.remove_path(Path::new(&repo_path))?;
        }
    }
    index.write()?;
    Ok(())
}

/// Stage every change in the project, but nothing elsewhere in the repository.
pub fn stage_all(project: &Path) -> Result<()> {
    let repo = ProjectRepo::require(project)?;
    let pathspec = repo.pathspec();
    let mut index = repo.repo.index()?;
    index.add_all(&pathspec, IndexAddOption::DEFAULT, None)?;
    index.update_all(&pathspec, None)?;
    index.write()?;
    Ok(())
}

/// Put the files' index entries back as they were at HEAD; the work tree is
/// untouched. Before the first commit this removes them from the index.
pub fn unstage(project: &Path, paths: &[ProjectPath]) -> Result<()> {
    let repo = ProjectRepo::require(project)?;
    reset(&repo, paths.iter().map(|path| repo.repo_path(path)).collect())
}

pub fn unstage_all(project: &Path) -> Result<()> {
    let repo = ProjectRepo::require(project)?;
    reset(&repo, repo.pathspec())
}

/// An empty `pathspec` resets everything.
fn reset(repo: &ProjectRepo, pathspec: Vec<String>) -> Result<()> {
    let head = repo.head_commit()?;
    repo.repo.reset_default(head.as_ref().map(git2::Commit::as_object), pathspec)?;
    Ok(())
}

/// Throw away the files' unstaged changes: tracked files go back to their
/// staged content, untracked ones are deleted. Staged changes are kept.
pub fn discard(project: &Path, paths: &[ProjectPath]) -> Result<()> {
    let repo = ProjectRepo::require(project)?;
    let index = repo.repo.index()?;
    let mut checkout = CheckoutBuilder::new();
    checkout.force().disable_pathspec_match(true);
    let mut any_tracked = false;
    for path in paths {
        let repo_path = repo.repo_path(path);
        if index.get_path(Path::new(&repo_path), 0).is_some() {
            checkout.path(&repo_path);
            any_tracked = true;
        } else if repo.repo.status_file(Path::new(&repo_path))?.is_wt_new() {
            // Only files git reports as untracked: never an ignored one.
            remove_if_present(&path.resolve(project))?;
        }
    }
    if any_tracked {
        repo.repo.checkout_index(None, Some(&mut checkout))?;
    }
    Ok(())
}

/// [`discard`] every unstaged change in the project.
pub fn discard_all(project: &Path) -> Result<()> {
    let Some(status) = status(project)? else { return Err(crate::Error::NotARepository) };
    let paths: Vec<_> = status.files.into_iter().filter(|file| file.unstaged.is_some()).map(|file| file.path).collect();
    discard(project, &paths)
}

fn remove_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::status::ChangeKind;
    use super::super::testing::{commit_all, init, write};
    use super::*;

    fn path(rel: &str) -> ProjectPath {
        ProjectPath::parse(rel).unwrap()
    }

    fn kinds(project: &Path) -> Vec<(String, Option<ChangeKind>, Option<ChangeKind>)> {
        let status = status(project).unwrap().unwrap();
        status.files.into_iter().map(|f| (f.path.to_string(), f.staged, f.unstaged)).collect()
    }

    #[test]
    fn stage_and_unstage_before_the_first_commit() {
        let (dir, _repo) = init();
        let p = dir.path();
        write(p, "a.yml", "a");
        stage(p, &[path("a.yml")]).unwrap();
        assert_eq!(kinds(p), [("a.yml".into(), Some(ChangeKind::Added), None)]);
        unstage(p, &[path("a.yml")]).unwrap();
        assert_eq!(kinds(p), [("a.yml".into(), None, Some(ChangeKind::Untracked))]);
    }

    #[test]
    fn stage_and_unstage_a_deletion() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "a.yml", "a");
        commit_all(&repo, "first");
        fs::remove_file(p.join("a.yml")).unwrap();
        stage(p, &[path("a.yml")]).unwrap();
        assert_eq!(kinds(p), [("a.yml".into(), Some(ChangeKind::Deleted), None)]);
        unstage(p, &[path("a.yml")]).unwrap();
        assert_eq!(kinds(p), [("a.yml".into(), None, Some(ChangeKind::Deleted))]);
    }

    #[test]
    fn stage_all_and_unstage_all_stay_in_the_project() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "api/a.yml", "a");
        write(p, "api/gone.yml", "g");
        write(p, "outside.md", "o");
        commit_all(&repo, "first");
        write(p, "api/a.yml", "changed");
        write(p, "api/new.yml", "n");
        fs::remove_file(p.join("api/gone.yml")).unwrap();
        write(p, "outside.md", "changed");
        let project = p.join("api");

        stage_all(&project).unwrap();
        assert_eq!(
            kinds(&project),
            [
                ("a.yml".into(), Some(ChangeKind::Modified), None),
                ("gone.yml".into(), Some(ChangeKind::Deleted), None),
                ("new.yml".into(), Some(ChangeKind::Added), None),
            ]
        );
        assert_eq!(status(&project).unwrap().unwrap().staged_outside, 0);
        assert_eq!(kinds(p).last().unwrap(), &("outside.md".into(), None, Some(ChangeKind::Modified)));

        unstage_all(&project).unwrap();
        assert!(kinds(&project).iter().all(|(_, staged, _)| staged.is_none()));
    }

    #[test]
    fn discard_restores_tracked_and_deletes_untracked_files() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "a.yml", "a");
        write(p, ".gitignore", "ignored.yml\n");
        commit_all(&repo, "first");
        write(p, "a.yml", "staged");
        stage(p, &[path("a.yml")]).unwrap();
        write(p, "a.yml", "staged, then edited");
        write(p, "new.yml", "n");
        write(p, "ignored.yml", "keep me");

        discard(p, &[path("a.yml"), path("new.yml"), path("ignored.yml")]).unwrap();
        assert_eq!(fs::read_to_string(p.join("a.yml")).unwrap(), "staged");
        assert!(!p.join("new.yml").exists());
        assert!(p.join("ignored.yml").exists());
        assert_eq!(kinds(p), [("a.yml".into(), Some(ChangeKind::Modified), None)]);
    }

    #[test]
    fn discard_all_keeps_staged_changes() {
        let (dir, repo) = init();
        let p = dir.path();
        write(p, "a.yml", "a");
        write(p, "b.yml", "b");
        commit_all(&repo, "first");
        write(p, "a.yml", "edited");
        write(p, "b.yml", "staged");
        stage(p, &[path("b.yml")]).unwrap();
        write(p, "new.yml", "n");

        discard_all(p).unwrap();
        assert_eq!(fs::read_to_string(p.join("a.yml")).unwrap(), "a");
        assert!(!p.join("new.yml").exists());
        assert_eq!(kinds(p), [("b.yml".into(), Some(ChangeKind::Modified), None)]);
    }
}
