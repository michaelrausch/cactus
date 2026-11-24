use std::path::PathBuf;

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
// The UI receives errors as their message; see the `Serialize` impl.
#[cfg_attr(feature = "specta", derive(specta::Type), specta(type = String))]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),
    /// The file could not be edited in place, so it was left untouched.
    #[error("{path}: can't update the file in place: {reason}")]
    Unpatchable { path: String, reason: String },
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid path: {0}")]
    InvalidScenarioPath(String),
    #[error("invalid environment name: {0:?}")]
    InvalidEnvName(String),
    #[error("invalid method: {0}")]
    InvalidMethod(String),
    #[error("invalid header name: {0}")]
    InvalidHeaderName(String),
    #[error("invalid value for header {0}")]
    InvalidHeaderValue(String),
    #[error("{}: {source}", path.display())]
    AttachmentUnreadable { path: PathBuf, source: std::io::Error },
    #[error("{0} already exists")]
    AlreadyExists(String),
    #[error("not a folder: {}", .0.display())]
    NotAFolder(PathBuf),
    #[error("not a Postman collection or OpenAPI spec")]
    UnrecognizedImportFormat,
    #[cfg(feature = "git")]
    #[error("git: {}", .0.message())]
    Git(#[from] git2::Error),
    #[error("not a git repository")]
    NotARepository,
    #[error("set user.name and user.email in your git config to commit")]
    GitIdentityMissing,
    #[error("commit message is empty")]
    EmptyCommitMessage,
    #[error("nothing staged to commit")]
    NothingToCommit,
    #[error("finish the {0} in progress first")]
    RepositoryBusy(String),
    #[error("make a first commit before creating branches")]
    NoCommitsYet,
    #[error("invalid branch name: {0:?}")]
    InvalidBranchName(String),
    #[error("switching to {0} would overwrite uncommitted changes; commit or discard them first")]
    CheckoutConflict(String),
    #[error("{0} is checked out, so it can't be deleted")]
    DeleteCurrentBranch(String),
    #[error("{0} has commits that aren't on the current branch")]
    BranchNotMerged(String),
}

/// The desktop UI receives errors as their display string.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
