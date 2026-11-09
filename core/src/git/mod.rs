//! The git repository the project is in: status, staging, diffs and commits.
//!
//! The project may be the repository's root or any folder inside it. Paths
//! in and out are project-relative [`ProjectPath`](crate::ProjectPath)s,
//! and only the project's files are listed or staged. Nothing here touches
//! the network.

mod branch;
mod commit;
mod diff;
mod repo;
mod stage;
mod status;
#[cfg(test)]
mod testing;

pub use branch::{BranchInfo, branches, create_branch, delete_branch, switch_branch};
pub use commit::{CommitSummary, GitIdentity, commit, configured_identity};
pub use diff::{DiffHunk, DiffLine, DiffLineKind, DiffSide, FileDiff, diff};
pub use repo::init;
pub use stage::{discard, discard_all, stage, stage_all, unstage, unstage_all};
pub use status::{ChangeKind, FileStatus, GitStatus, status};
