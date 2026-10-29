//! The env layers and how they merge.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::storage::{read_json, write_json};
use crate::{Error, Result, layout, state};

pub type Env = BTreeMap<String, String>;

/// One env layer file. Later layers override earlier ones; see [`resolve`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum EnvFile {
    /// `env.json`, shared and committed.
    Base,
    /// `env.<name>.json`, one of which is active.
    Named(String),
    /// `.cactus/secrets.json`, never committed.
    Secrets,
    /// `.cactus/captured.json`, written by `capture` rules.
    Captured,
}

impl EnvFile {
    /// Whether this layer lives in the local-only `.cactus/` folder.
    fn is_local(&self) -> bool {
        matches!(self, Self::Secrets | Self::Captured)
    }

    fn path(&self, project: &Path) -> Result<PathBuf> {
        Ok(match self {
            Self::Base => project.join(layout::BASE_ENV_FILE),
            Self::Named(name) => {
                validate_env_name(name)?;
                project.join(layout::named_env_file_name(name))
            }
            Self::Secrets => layout::secrets_file(project),
            Self::Captured => layout::captured_file(project),
        })
    }
}

/// Env names become file names, so they are limited to `[A-Za-z0-9_-]+`.
pub fn validate_env_name(name: &str) -> Result<()> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    if name.is_empty() || !name.chars().all(allowed) {
        return Err(Error::InvalidEnvName(name.to_string()));
    }
    Ok(())
}

pub fn load(project: &Path, file: &EnvFile) -> Result<Env> {
    read_json(&file.path(project)?)
}

pub fn save(project: &Path, file: &EnvFile, env: &Env) -> Result<()> {
    if file.is_local() {
        layout::ensure_workspace(project)?;
    }
    write_json(&file.path(project)?, env)
}

/// Names of the `env.<name>.json` files at the project root, sorted.
pub fn list_named(project: &Path) -> Result<Vec<String>> {
    let mut names: Vec<String> = fs::read_dir(project)?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter_map(|file_name| layout::named_env_from_file_name(&file_name).map(str::to_string))
        .filter(|name| validate_env_name(name).is_ok())
        .collect();
    names.sort();
    Ok(names)
}

/// The active named env, if its file still exists.
pub fn active(project: &Path) -> Option<String> {
    let name = state::load(project).env?;
    EnvFile::Named(name.clone()).path(project).ok()?.exists().then_some(name)
}

pub fn set_active(project: &Path, name: Option<String>) -> Result<()> {
    if let Some(name) = &name {
        validate_env_name(name)?;
    }
    let mut workspace_state = state::load(project);
    workspace_state.env = name;
    state::save(project, &workspace_state)
}

/// Merge every layer, lowest precedence first: base, the named env, secrets,
/// captured. `named` picks the env; `None` means the active one.
pub fn resolve(project: &Path, named: Option<&str>) -> Result<Env> {
    let named = named.map(str::to_string).or_else(|| active(project));
    layer_order(named).iter().try_fold(Env::new(), |mut merged, file| {
        merged.extend(load(project, file)?);
        Ok(merged)
    })
}

fn layer_order(named: Option<String>) -> Vec<EnvFile> {
    let mut layers = vec![EnvFile::Base];
    layers.extend(named.map(EnvFile::Named));
    layers.extend([EnvFile::Secrets, EnvFile::Captured]);
    layers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::merge_captured;

    fn env(pairs: &[(&str, &str)]) -> Env {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    #[test]
    fn layers_override_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        assert!(resolve(p, None).unwrap().is_empty());
        save(p, &EnvFile::Base, &env(&[("a", "base"), ("b", "base"), ("c", "base"), ("d", "base")])).unwrap();
        save(p, &EnvFile::Named("dev".into()), &env(&[("b", "dev"), ("c", "dev"), ("d", "dev")])).unwrap();
        save(p, &EnvFile::Named("prod".into()), &env(&[("b", "prod")])).unwrap();
        save(p, &EnvFile::Secrets, &env(&[("c", "secret"), ("d", "secret")])).unwrap();
        merge_captured(p, &env(&[("d", "captured")])).unwrap();

        assert_eq!(list_named(p).unwrap(), ["dev", "prod"]);
        assert_eq!(resolve(p, None).unwrap()["b"], "base");
        set_active(p, Some("dev".into())).unwrap();
        assert_eq!(active(p).as_deref(), Some("dev"));
        let merged = resolve(p, None).unwrap();
        assert_eq!([&merged["a"], &merged["b"], &merged["c"], &merged["d"]], ["base", "dev", "secret", "captured"]);
        assert_eq!(resolve(p, Some("prod")).unwrap()["b"], "prod");
        assert_eq!(fs::read_to_string(p.join(".cactus/.gitignore")).unwrap(), "*\n");
        assert!(save(p, &EnvFile::Named("../x".into()), &Env::new()).is_err());
    }
}
