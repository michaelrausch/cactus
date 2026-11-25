//! Core's single error type. Messages are user-facing: the app and the CLI
//! show them as-is, so word new variants accordingly.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
// The UI receives errors as their message; see the `Serialize` impl.
#[cfg_attr(feature = "specta", derive(specta::Type), specta(type = String))]
pub enum Error {
    /// An I/O failure with no one file to blame; see [`Error::File`].
    #[error("io: {0}")]
    Io(#[from] io::Error),
    /// Reading or writing `path` failed.
    #[error("{}: {source}", path.display())]
    File { path: PathBuf, source: io::Error },
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
    InvalidPath(String),
    #[error("invalid environment name: {0:?}")]
    InvalidEnvName(String),
    #[error("invalid critter name: {0:?}; use letters, digits, - and _")]
    InvalidCritterName(String),
    #[error("invalid flow name: {0:?}; use letters, digits, - and _")]
    InvalidFlowName(String),
    #[error("no critter {0:?}")]
    CritterNotFound(String),
    #[error("critter {critter:?} has no flow {flow:?}")]
    FlowNotFound { critter: String, flow: String },
    #[error("every: {0:?}: use e.g. 30s, 5m or 1h; minimum 30s")]
    InvalidInterval(String),
    #[error("the request has no URL")]
    MissingUrl,
    /// A flow tool step failed; the message says why.
    #[error("{0}")]
    Tool(String),
    /// Getting or refreshing an OAuth2 token failed; the message says why.
    #[error("OAuth2: {0}")]
    OAuth2(String),
    /// The system browser couldn't be opened; the message says why.
    #[error("couldn't open the browser: {0}")]
    Browser(String),
    #[error("invalid URL {url:?}: {reason}")]
    InvalidUrl { url: String, reason: String },
    #[error("invalid method: {0}")]
    InvalidMethod(String),
    #[error("invalid header name: {0}")]
    InvalidHeaderName(String),
    #[error("invalid value for header {0}")]
    InvalidHeaderValue(String),
    #[error("attachment {0:?} is outside the project; give a path relative to the project folder")]
    AttachmentOutsideProject(String),
    #[error("{0} already exists")]
    AlreadyExists(String),
    #[error("not a folder: {}", .0.display())]
    NotAFolder(PathBuf),
    #[error("not a Postman collection or OpenAPI spec")]
    UnrecognizedImportFormat,
    #[error("locked")]
    Locked,
    #[error("wrong password")]
    WrongPassword,
    #[error("password is empty")]
    EmptyPassword,
    #[error("wrong code")]
    WrongCode,
    #[error("set a password first")]
    NoPassword,
    #[error("{}: can't decrypt", .0.display())]
    Undecryptable(PathBuf),
    #[error("encryption: {0}")]
    Crypto(String),
    #[error("invalid authenticator secret")]
    InvalidTotpSecret,
    #[error("port must be 1024–65535, not {0}")]
    InvalidPort(u16),
    #[cfg(feature = "git")]
    #[error("git: {}", .0.message())]
    Git(#[from] git2::Error),
    #[cfg(feature = "git")]
    #[error("not a git repository")]
    NotARepository,
    #[cfg(feature = "git")]
    #[error("set your name and email in Settings → Git, or user.name and user.email in git config, to commit")]
    GitIdentityMissing,
    #[cfg(feature = "git")]
    #[error("commit message is empty")]
    EmptyCommitMessage,
    #[cfg(feature = "git")]
    #[error("nothing staged to commit")]
    NothingToCommit,
    #[cfg(feature = "git")]
    #[error("finish the {0} in progress first")]
    RepositoryBusy(String),
    #[cfg(feature = "git")]
    #[error("make a first commit before creating branches")]
    NoCommitsYet,
    #[cfg(feature = "git")]
    #[error("invalid branch name: {0:?}")]
    InvalidBranchName(String),
    #[cfg(feature = "git")]
    #[error("switching to {0} would overwrite uncommitted changes; commit or discard them first")]
    CheckoutConflict(String),
    #[cfg(feature = "git")]
    #[error("{0} is checked out, so it can't be deleted")]
    DeleteCurrentBranch(String),
    #[cfg(feature = "git")]
    #[error("{0} has commits that aren't on the current branch")]
    BranchNotMerged(String),
}

/// The desktop UI receives errors as their display string.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Error {
    /// Attach `path` to an I/O error: `.map_err(Error::file(path))`.
    pub(crate) fn file(path: &Path) -> impl FnOnce(io::Error) -> Self + '_ {
        move |source| Self::File { path: path.to_path_buf(), source }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
