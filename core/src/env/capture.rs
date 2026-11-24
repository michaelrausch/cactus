use std::path::Path;

use super::layers::{Env, EnvFile, load, save};
use crate::Result;

/// Merge newly captured values into `.cactus/captured.json`.
pub fn merge_captured(project: &Path, values: &Env) -> Result<()> {
    let mut captured = load(project, &EnvFile::Captured)?;
    captured.extend(values.iter().map(|(k, v)| (k.clone(), v.clone())));
    save(project, &EnvFile::Captured, &captured)
}
