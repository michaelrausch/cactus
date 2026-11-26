//! Throwaway repositories for the git tests.

use std::fs;
use std::path::Path;

use git2::{Repository, RepositoryInitOptions, Signature};
use tempfile::TempDir;

/// An empty repository on `main` with an identity of its own, so tests
/// don't depend on the machine's git config.
pub fn init() -> (TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init_opts(dir.path(), RepositoryInitOptions::new().initial_head("main")).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    (dir, repo)
}

pub fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// `git add -A`.
pub fn stage_everything(repo: &Repository) {
    let mut index = repo.index().unwrap();
    index.add_all(["*"], git2::IndexAddOption::DEFAULT, None).unwrap();
    index.update_all(["*"], None).unwrap();
    index.write().unwrap();
}

/// Stage everything and commit it, as `git add -A && git commit` would.
pub fn commit_all(repo: &Repository, message: &str) {
    stage_everything(repo);
    let mut index = repo.index().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = Signature::now("Test", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<_> = parent.iter().collect();
    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parents).unwrap();
}
