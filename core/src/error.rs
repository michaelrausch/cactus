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
    Yaml(#[from] serde_yaml::Error),
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
    #[error("{0} body must be an object of fields")]
    BodyNotAnObject(&'static str),
    #[error("{}: {source}", path.display())]
    AttachmentUnreadable { path: PathBuf, source: std::io::Error },
    #[error("{0} already exists")]
    AlreadyExists(String),
    #[error("not a folder: {}", .0.display())]
    NotAFolder(PathBuf),
    #[error("not a Postman collection or OpenAPI spec")]
    UnrecognizedImportFormat,
}

/// The desktop UI receives errors as their display string.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
