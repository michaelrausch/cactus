//! The env layers and how they merge.

use std::collections::{BTreeMap, HashSet};
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
            let dir = layout::environments_dir(project);
            fs::create_dir_all(&dir).map_err(Error::file(&dir))?;
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
    variables.iter().filter(|v| v.is_active()).map(|v| (v.name.to_string(), v.resolved_value())).collect()
}

/// Make the active variables match `env`: change values in place, drop the
/// missing, append the new. Untouched variables stay exactly as written, and
/// a name written twice keeps both entries, each given the new value.
fn merge_values(variables: &mut Vec<Variable>, env: &Env) {
    variables.retain_mut(|variable| {
        if !variable.is_active() {
            return true;
        }
        let Some(value) = env.get(variable.name.as_str()) else { return false };
        if variable.resolved_value() != *value {
            variable.value = Some(VariableValue::Text(value.clone()));
        }
        true
    });
    let written: HashSet<&str> = variables.iter().filter(|v| v.is_active()).map(|v| v.name.as_str()).collect();
    let added = env.iter().filter(|(name, _)| !written.contains(name.as_str()));
    variables.extend(added.map(|(name, value)| Variable::new(name.clone(), value.clone())).collect::<Vec<_>>());
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
    fs::rename(&source, &target).map_err(Error::file(&source))?;
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
    let file = layout::environment_file(project, name);
    fs::remove_file(&file).map_err(Error::file(&file))?;
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
    let root = collection::load_root(project)?;
    merge_layers(project, named, &root.request.variables, &[], &[])
}

/// What a request in folder `dir` inherits and the variables it sees.
pub struct RequestScope {
    /// The collection's defaults, then each folder's down to `dir`; see
    /// [`collection::defaults_chain`].
    pub inherited: Vec<RequestDefaults>,
    pub vars: Env,
}

/// [`resolve`] for one request in folder `dir`: the variables of its folders
/// (outermost first) and then its own go between the named env and secrets,
/// so a folder or request can override an environment but local secrets
/// still win.
pub fn resolve_for_request(
    project: &Path,
    named: Option<&str>,
    dir: &str,
    request_variables: &[Variable],
) -> Result<RequestScope> {
    let inherited = collection::defaults_chain(project, dir)?;
    let (collection_variables, folders) = match inherited.split_first() {
        Some((root, folders)) => (root.variables.as_slice(), folders),
        None => (&[][..], &[][..]),
    };
    let vars = merge_layers(project, named, collection_variables, folders, request_variables)?;
    Ok(RequestScope { inherited, vars })
}

/// The names of every variable a request in `dir` can use with the active
/// env, sorted.
pub fn names_in_scope(project: &Path, dir: &str) -> Result<Vec<String>> {
    Ok(resolve_for_request(project, None, dir, &[])?.vars.into_keys().collect())
}

fn merge_layers(
    project: &Path,
    named: Option<&str>,
    collection_variables: &[Variable],
    folders: &[RequestDefaults],
    request_variables: &[Variable],
) -> Result<Env> {
    let named = named.map(str::to_string).or_else(|| active(project));
    let mut merged = active_values(collection_variables);
    if let Some(name) = named {
        merged.extend(load(project, &EnvFile::Named(name))?);
    }
    for folder in folders {
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

        collection::ensure_folder(p, "f").unwrap();
        let folder_file = layout::folder_file(p, "f");
        let mut folder: collection::Folder = collection::read(&folder_file).unwrap();
        folder.request.variables.push(Variable::new("c", "folder"));
        collection::write(&folder_file, &folder).unwrap();
        assert_eq!(resolve_for_request(p, None, "f", &[]).unwrap().vars["c"], "folder");
        let request = [Variable::new("c", "request"), Variable::new("d", "request")];
        let scope = resolve_for_request(p, None, "f", &request[..1]).unwrap();
        assert_eq!((scope.inherited.len(), scope.vars["c"].as_str()), (2, "request"));
        assert_eq!(resolve_for_request(p, None, "f", &request).unwrap().vars["d"], "secret");
        assert_eq!(names_in_scope(p, "f").unwrap(), ["a", "b", "c", "d", "e"]);
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
    fn saving_keeps_every_entry_of_a_repeated_name() {
        let mut variables = vec![Variable::new("a", "1"), Variable::new("a", "2"), Variable::new("gone", "x")];
        merge_values(&mut variables, &env(&[("a", "3"), ("new", "4")]));
        let pairs: Vec<_> = variables.iter().map(|v| (v.name.as_str(), v.resolved_value())).collect();
        assert_eq!(pairs, [("a", "3".into()), ("a", "3".into()), ("new", "4".into())]);
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
