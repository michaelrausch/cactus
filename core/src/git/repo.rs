//! The git repository a project lives in, which may be the project folder
//! itself or any folder above it.

use std::fs;
use std::path::Path;

use git2::{Commit, ErrorCode, Repository, RepositoryState, Tree};

use crate::{Error, ProjectPath, Result};

const SEPARATOR: char = '/';

pub(super) struct ProjectRepo {
    pub repo: Repository,
    /// The project folder inside the work tree: `""` at its root, otherwise
    /// `/`-separated with a trailing `/`.
    prefix: String,
}

impl ProjectRepo {
    /// The repository containing `project`, or `None` if there is none or it
    /// has no work tree.
    pub fn open(project: &Path) -> Result<Option<Self>> {
        let repo = match Repository::discover(project) {
            Ok(repo) => repo,
            Err(e) if e.code() == ErrorCode::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let Some(workdir) = repo.workdir() else { return Ok(None) };
        // Both canonical, so they share a form (`\\?\C:\…` on Windows).
        let workdir = fs::canonicalize(workdir)?;
        let Ok(relative) = fs::canonicalize(project)?.strip_prefix(&workdir).map(Path::to_path_buf) else {
            return Ok(None);
        };
        let mut prefix = relative.to_string_lossy().replace('\\', "/");
        if !prefix.is_empty() {
            prefix.push(SEPARATOR);
        }
        Ok(Some(Self { repo, prefix }))
    }

    pub fn require(project: &Path) -> Result<Self> {
        Self::open(project)?.ok_or(Error::NotARepository)
    }

    /// `path` relative to the work tree, as git names it.
    pub fn repo_path(&self, path: &ProjectPath) -> String {
        format!("{}{path}", self.prefix)
    }

    /// The project-relative path of a work-tree path, if it is in the project.
    pub fn project_path(&self, repo_path: &str) -> Option<ProjectPath> {
        ProjectPath::parse(repo_path.strip_prefix(&self.prefix)?).ok()
    }

    /// Limits an operation to the project: nothing at the work-tree root,
    /// where everything is in the project, otherwise its folder.
    pub fn pathspec(&self) -> Vec<String> {
        let folder = self.prefix.trim_end_matches(SEPARATOR);
        if folder.is_empty() { Vec::new() } else { vec![folder.to_string()] }
    }

    /// `None` before the first commit.
    pub fn head_commit(&self) -> Result<Option<Commit<'_>>> {
        match self.repo.head() {
            Ok(head) => Ok(Some(head.peel_to_commit()?)),
            Err(e) if matches!(e.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn head_tree(&self) -> Result<Option<Tree<'_>>> {
        self.head_commit()?.map(|commit| commit.tree()).transpose().map_err(Error::from)
    }

    /// The checked-out branch, also before its first commit; `None` when
    /// HEAD is detached.
    pub fn branch(&self) -> Option<String> {
        let head = self.repo.find_reference("HEAD").ok()?;
        head.symbolic_target()?.strip_prefix("refs/heads/").map(str::to_string)
    }

    /// The operation in progress that a commit would get in the way of.
    pub fn busy(&self) -> Option<&'static str> {
        match self.repo.state() {
            RepositoryState::Clean | RepositoryState::Bisect => None,
            RepositoryState::Merge => Some("merge"),
            RepositoryState::Revert | RepositoryState::RevertSequence => Some("revert"),
            RepositoryState::CherryPick | RepositoryState::CherryPickSequence => Some("cherry-pick"),
            RepositoryState::Rebase | RepositoryState::RebaseInteractive | RepositoryState::RebaseMerge => {
                Some("rebase")
            }
            RepositoryState::ApplyMailbox | RepositoryState::ApplyMailboxOrRebase => Some("patch"),
        }
    }
}

/// Make the project folder a repository of its own, on git's default
/// branch. Refused when it is already in one.
pub fn init(project: &Path) -> Result<()> {
    if ProjectRepo::open(project)?.is_some() {
        return Err(Error::AlreadyExists("a git repository".into()));
    }
    Repository::init(project)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::testing::{init, write};
    use super::*;

    #[test]
    fn not_a_repository() {
        let dir = tempfile::tempdir().unwrap();
        // A temp dir could sit inside someone's repo; only check what we can.
        if Repository::discover(dir.path()).is_err() {
            assert!(ProjectRepo::open(dir.path()).unwrap().is_none());
        }
    }

    #[test]
    fn init_once() {
        let dir = tempfile::tempdir().unwrap();
        if Repository::discover(dir.path()).is_ok() {
            return; // The temp dir is inside someone's repository.
        }
        super::init(dir.path()).unwrap();
        assert!(ProjectRepo::open(dir.path()).unwrap().is_some());
        assert!(matches!(super::init(dir.path()), Err(Error::AlreadyExists(_))));
    }

    #[test]
    fn maps_paths_of_a_project_in_a_subfolder() {
        let (dir, _repo) = init();
        write(dir.path(), "api/collection/opencollection.yml", "");
        let repo = ProjectRepo::require(&dir.path().join("api").join("collection")).unwrap();
        let path = ProjectPath::parse("auth\\login.yml").unwrap();
        assert_eq!(repo.repo_path(&path), "api/collection/auth/login.yml");
        assert_eq!(repo.project_path("api/collection/auth/login.yml"), Some(path));
        assert_eq!(repo.project_path("api/other.yml"), None);
        assert_eq!(repo.pathspec(), ["api/collection"]);
    }

    #[test]
    fn project_at_the_root() {
        let (dir, _repo) = init();
        let repo = ProjectRepo::require(dir.path()).unwrap();
        assert_eq!(repo.repo_path(&ProjectPath::parse("a.yml").unwrap()), "a.yml");
        assert_eq!(repo.pathspec(), Vec::<String>::new());
        assert_eq!(repo.branch().as_deref(), Some("main"));
        assert!(repo.head_commit().unwrap().is_none());
    }
}
