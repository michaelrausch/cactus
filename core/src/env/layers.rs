//! The env layers and how they merge.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::collection::{self, Environment, RequestDefaults, Variable, VariableValue};
use crate::storage::{read_json, write_json};
use crate::{Error, Result, layout, state};

pub type Env = BTreeMap<String, String>;

/// One env layer. Later layers override earlier ones; see [`resolve`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum EnvFile {
    /// The collection's variables in `opencollection.yml`, shared and committed.
    Base,
    /// `environments/<name>.yml`, one of which is active.
    Named(String),
    /// `.cactus/secrets.json`, never committed.
    Secrets,
    /// `.cactus/captured.json`, written by set-variable actions.
    Captured,
}

/// Env names become file names, so they are limited to `[A-Za-z0-9_-]+`.
pub fn validate_env_name(name: &str) -> Result<()> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    if name.is_empty() || !name.chars().all(allowed) {
        return Err(Error::InvalidEnvName(name.to_string()));
    }
    Ok(())
}

/// The layer's variables that take part in substitution: secret and
/// disabled variables are left out.
pub fn load(project: &Path, file: &EnvFile) -> Result<Env> {
    Ok(match file {
        EnvFile::Base => active_values(&collection::load_root(project)?.request.variables),
        EnvFile::Named(name) => active_values(&load_environment(project, name)?.variables),
        EnvFile::Secrets => read_json(&layout::secrets_file(project))?,
        EnvFile::Captured => read_json(&layout::captured_file(project))?,
    })
}

/// Replace the layer's variables with `env`. In YAML layers the file is
/// edited in place, and secret and disabled variables stay as they are.
pub fn save(project: &Path, file: &EnvFile, env: &Env) -> Result<()> {
    match file {
        EnvFile::Base => {
            let mut root = collection::load_root(project)?;
            merge_values(&mut root.request.variables, env);
            collection::save_root(project, &root)
        }
        EnvFile::Named(name) => {
            let mut environment = load_environment(project, name)?;
            merge_values(&mut environment.variables, env);
            fs::create_dir_all(layout::environments_dir(project))?;
            collection::write(&layout::environment_file(project, name), &environment)
        }
        EnvFile::Secrets => {
            layout::ensure_workspace(project)?;
            write_json(&layout::secrets_file(project), env)
        }
        EnvFile::Captured => {
            layout::ensure_workspace(project)?;
            write_json(&layout::captured_file(project), env)
        }
    }
}

fn load_environment(project: &Path, name: &str) -> Result<Environment> {
    validate_env_name(name)?;
    let file = layout::environment_file(project, name);
    Ok(collection::read_if_exists(&file)?.unwrap_or_else(|| Environment { name: name.into(), variables: Vec::new() }))
}

fn active_values(variables: &[Variable]) -> Env {
    variables.iter().filter(|v| v.is_active()).map(|v| (v.name.clone(), v.resolved_value())).collect()
}

/// Make the active variables match `env`: change values in place, drop the
/// missing, append the new. Untouched variables stay exactly as written.
fn merge_values(variables: &mut Vec<Variable>, env: &Env) {
    let mut remaining = env.clone();
    variables.retain_mut(|variable| {
        if !variable.is_active() {
            return true;
        }
        let Some(value) = remaining.remove(&variable.name) else { return false };
        if variable.resolved_value() != value {
            variable.value = Some(VariableValue::Text(value));
        }
        true
    });
    variables.extend(remaining.into_iter().map(|(name, value)| Variable::new(name, value)));
}

/// Names of the environments in `environments/`, sorted.
pub fn list_named(project: &Path) -> Result<Vec<String>> {
    let Ok(entries) = fs::read_dir(layout::environments_dir(project)) else { return Ok(Vec::new()) };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter_map(|file_name| layout::environment_from_file_name(&file_name).map(str::to_string))
        .filter(|name| validate_env_name(name).is_ok())
        .collect();
    names.sort();
    Ok(names)
}

/// Rename environment `from` to `to`: its file and the `name` inside it,
/// edited in place. The active env follows the rename.
pub fn rename_named(project: &Path, from: &str, to: &str) -> Result<()> {
    validate_env_name(from)?;
    validate_env_name(to)?;
    if from == to {
        return Ok(());
    }
    let source = layout::environment_file(project, from);
    let target = layout::environment_file(project, to);
    if target.exists() {
        return Err(Error::AlreadyExists(format!("environment {to}")));
    }
    fs::rename(&source, &target)?;
    let mut environment = load_environment(project, to)?;
    environment.name = to.into();
    collection::write(&target, &environment)?;
    if state::load(project).env.as_deref() == Some(from) {
        set_active(project, Some(to.into()))?;
    }
    Ok(())
}

/// Delete environment `name`; if it was active, no environment is.
pub fn delete_named(project: &Path, name: &str) -> Result<()> {
    validate_env_name(name)?;
    fs::remove_file(layout::environment_file(project, name))?;
    if state::load(project).env.as_deref() == Some(name) {
        set_active(project, None)?;
    }
    Ok(())
}

/// The active named env, if its file still exists.
pub fn active(project: &Path) -> Option<String> {
    let name = state::load(project).env?;
    (validate_env_name(&name).is_ok() && layout::environment_file(project, &name).exists()).then_some(name)
}

pub fn set_active(project: &Path, name: Option<String>) -> Result<()> {
    if let Some(name) = &name {
        validate_env_name(name)?;
    }
    let mut workspace_state = state::load(project);
    workspace_state.env = name;
    state::save(project, &workspace_state)
}

/// Merge every layer, lowest precedence first: the collection's variables,
/// the named env, secrets, captured. `named` picks the env; `None` means
/// the active one.
pub fn resolve(project: &Path, named: Option<&str>) -> Result<Env> {
    resolve_for_request(project, named, &[], &[])
}

/// [`resolve`] for one request: the variables of its folders (`inherited`
/// past the collection's own, outermost first) and then its own go between
/// the named env and secrets, so a folder or request can override an
/// environment but local secrets still win.
pub fn resolve_for_request(
    project: &Path,
    named: Option<&str>,
    inherited: &[RequestDefaults],
    request_variables: &[Variable],
) -> Result<Env> {
    let named = named.map(str::to_string).or_else(|| active(project));
    let mut merged = load(project, &EnvFile::Base)?;
    if let Some(name) = named {
        merged.extend(load(project, &EnvFile::Named(name))?);
    }
    for folder in inherited.iter().skip(1) {
        merged.extend(active_values(&folder.variables));
    }
    merged.extend(active_values(request_variables));
    merged.extend(load(project, &EnvFile::Secrets)?);
    merged.extend(load(project, &EnvFile::Captured)?);
    Ok(merged)
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
        save(p, &EnvFile::Base, &env(&[("a", "base"), ("b", "base"), ("c", "base"), ("d", "base"), ("e", "base")]))
            .unwrap();
        save(p, &EnvFile::Named("dev".into()), &env(&[("b", "dev"), ("c", "dev"), ("d", "dev"), ("e", "dev")]))
            .unwrap();
        save(p, &EnvFile::Named("prod".into()), &env(&[("b", "prod")])).unwrap();
        save(p, &EnvFile::Secrets, &env(&[("d", "secret"), ("e", "secret")])).unwrap();
        merge_captured(p, &env(&[("e", "captured")])).unwrap();

        assert_eq!(list_named(p).unwrap(), ["dev", "prod"]);
        assert_eq!(resolve(p, None).unwrap()["b"], "base");
        set_active(p, Some("dev".into())).unwrap();
        assert_eq!(active(p).as_deref(), Some("dev"));
        let merged = resolve(p, None).unwrap();
        assert_eq!([&merged["a"], &merged["b"], &merged["d"], &merged["e"]], ["base", "dev", "secret", "captured"]);
        assert_eq!(resolve(p, Some("prod")).unwrap()["b"], "prod");
        assert_eq!(fs::read_to_string(p.join(".cactus/.gitignore")).unwrap(), "*\n");
        assert!(save(p, &EnvFile::Named("../x".into()), &Env::new()).is_err());

        let folder = RequestDefaults { variables: vec![Variable::new("c", "folder")], ..Default::default() };
        let request = [Variable::new("c", "request"), Variable::new("d", "request")];
        let for_request =
            resolve_for_request(p, None, &[RequestDefaults::default(), folder.clone()], &request[..1]).unwrap();
        assert_eq!(for_request["c"], "request");
        let for_request = resolve_for_request(p, None, &[RequestDefaults::default(), folder], &request).unwrap();
        assert_eq!(for_request["d"], "secret");
    }

    #[test]
    fn rename_and_delete_named() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        fs::create_dir_all(p.join("environments")).unwrap();
        fs::write(p.join("environments/dev.yml"), "name: dev\n# keep me\nvariables:\n  - name: a\n    value: '1'\n")
            .unwrap();
        save(p, &EnvFile::Named("prod".into()), &env(&[("b", "2")])).unwrap();
        set_active(p, Some("dev".into())).unwrap();

        assert!(rename_named(p, "dev", "prod").is_err());
        rename_named(p, "dev", "staging").unwrap();
        assert_eq!(list_named(p).unwrap(), ["prod", "staging"]);
        assert_eq!(
            fs::read_to_string(p.join("environments/staging.yml")).unwrap(),
            "name: staging\n# keep me\nvariables:\n  - name: a\n    value: '1'\n"
        );
        assert_eq!(active(p).as_deref(), Some("staging"));

        delete_named(p, "staging").unwrap();
        assert_eq!(list_named(p).unwrap(), ["prod"]);
        assert_eq!(active(p), None);
        assert!(delete_named(p, "../x").is_err());
    }

    #[test]
    fn saving_keeps_secrets_disabled_and_comments() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        fs::create_dir_all(p.join("environments")).unwrap();
        let file = p.join("environments/dev.yml");
        fs::write(
            &file,
            "name: dev\nvariables:\n  # Where the API lives.\n  - name: baseUrl\n    value: http://localhost\n  - name: token\n    secret: true\n  - name: off\n    value: x\n    disabled: true\n  - name: gone\n    value: y\n",
        )
        .unwrap();
        assert_eq!(
            load(p, &EnvFile::Named("dev".into())).unwrap(),
            env(&[("baseUrl", "http://localhost"), ("gone", "y")])
        );
        save(p, &EnvFile::Named("dev".into()), &env(&[("baseUrl", "http://dev"), ("new", "1")])).unwrap();
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "name: dev\nvariables:\n  # Where the API lives.\n  - name: baseUrl\n    value: http://dev\n  - name: token\n    secret: true\n  - name: off\n    value: x\n    disabled: true\n  - name: new\n    value: '1'\n",
        );
    }
}
