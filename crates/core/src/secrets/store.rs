//! Reading and writing secrets files: the only code that touches them.

use std::path::{Path, PathBuf};

use super::vault::{read_json, write_json};
use crate::critter::validate_critter_id;
use crate::env::{Env, ensure_critter_workspace};
use crate::{Result, layout};

/// The file name, in `.cactus/` and in each critter's folder there.
pub const SECRETS_FILE: &str = "secrets.json";

/// Every secrets file a project can have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretFile {
    /// `.cactus/secrets.json`.
    Project,
    /// `.cactus/critters/<id>/secrets.json`.
    Critter(String),
}

impl SecretFile {
    fn path(&self, project: &Path) -> Result<PathBuf> {
        Ok(match self {
            Self::Project => layout::workspace_dir(project).join(SECRETS_FILE),
            Self::Critter(id) => {
                validate_critter_id(id)?;
                layout::critter_workspace_dir(project, id).join(SECRETS_FILE)
            }
        })
    }
}

/// The file's values.
pub fn read(project: &Path, file: &SecretFile) -> Result<Env> {
    read_json(project, &file.path(project)?)
}

/// The file's variable names.
pub fn names(project: &Path, file: &SecretFile) -> Result<Vec<String>> {
    Ok(read(project, file)?.into_keys().collect())
}

/// Replace the file's values with `env`.
pub fn write(project: &Path, file: &SecretFile, env: &Env) -> Result<()> {
    match file {
        SecretFile::Project => layout::ensure_workspace(project).map(drop)?,
        SecretFile::Critter(id) => ensure_critter_workspace(project, id)?,
    }
    write_json(project, &file.path(project)?, env)
}
