//! The env layers and how they merge.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::capture::{load_context, replace_captured, replace_context_values};
use crate::collection::{self, Environment, RequestDefaults, Variable, VariableValue};
use crate::critter;
use crate::secrets::{self, SecretFile};
use crate::storage::read_json;
use crate::{Error, Result, layout, state};

pub type Env = BTreeMap<String, String>;

/// One env layer. Later layers override earlier ones; see [`resolve_for_request`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "camelCase")]
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
    /// The variables in `critters/<id>/critter.yml`.
    Critter(String),
    /// `.cactus/critters/<id>/secrets.json`.
    CritterSecrets(String),
    /// The values of `.cactus/critters/<id>/context.json`.
    CritterContext(String),
}

/// Which env, critter and step overrides a request is resolved with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    /// The named env; `None` is no env.
    pub env: Option<String>,
    /// The critter sending; `None` is no critter.
    pub critter: Option<String>,
    /// A flow step's `with`, above everything else.
    pub overrides: Env,
}

impl Scope {
    /// What an interactive send uses: the active env and critter.
    pub fn active(project: &Path) -> Self {
        Self { env: active(project), critter: critter::active(project), overrides: Env::new() }
    }

    /// Env `named`, or the active one, with no critter.
    pub fn env(project: &Path, named: Option<&str>) -> Self {
        Self { env: named.map(str::to_string).or_else(|| active(project)), ..Self::default() }
    }

    #[must_use]
    pub fn with_critter(self, critter: Option<String>) -> Self {
        Self { critter, ..self }
    }
}

/// Where a variable's value comes from: the highest layer that sets it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum VarSource {
    Collection,
    Environment {
        name: String,
    },
    Folder {
        dir: String,
    },
    Request,
    Secret,
    Captured,
    Critter,
    CritterSecret,
    CritterContext {
        from: String,
        // A JS `number`, accepting precision loss: epoch millis stay far below 2^53.
        #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
        at: u64,
    },
    Step,
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
        EnvFile::Secrets => secrets::read(project, &SecretFile::Project)?,
        EnvFile::Captured => read_json(&layout::captured_file(project))?,
        EnvFile::Critter(id) => active_values(&critter::load(project, id)?.variables),
        EnvFile::CritterSecrets(id) => secrets::read(project, &SecretFile::Critter(id.clone()))?,
        EnvFile::CritterContext(id) => load_context(project, id)?.into_iter().map(|(k, e)| (k, e.value)).collect(),
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
        EnvFile::Secrets => secrets::write(project, &SecretFile::Project, env),
        EnvFile::Captured => replace_captured(project, env),
        EnvFile::Critter(id) => {
            let mut critter = critter::load(project, id)?;
            merge_values(&mut critter.variables, env);
            critter::save(project, id, &critter)
        }
        EnvFile::CritterSecrets(id) => secrets::write(project, &SecretFile::Critter(id.clone()), env),
        EnvFile::CritterContext(id) => replace_context_values(project, id, env),
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
    Ok(merge(layers(project, &Scope::env(project, named), &root.request.variables, &[], &[], Need::Values)?))
}

/// What [`layers`] must read from secrets files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Need {
    /// The values, which fails with [`Error::SecretsLocked`] while a sealed
    /// value can't be opened.
    Values,
    /// Only the names, with empty values: works while locked.
    Names,
}

/// What a request in folder `dir` inherits and the variables it sees.
pub struct RequestScope {
    /// The collection's defaults, then each folder's down to `dir`; see
    /// [`collection::defaults_chain`].
    pub inherited: Vec<RequestDefaults>,
    pub vars: Env,
}

/// The variables of one request in folder `dir`, merged from every layer of
/// `scope` (see [`layers`]). While secrets are locked and a layer holds a
/// sealed value this is [`Error::SecretsLocked`], so nothing is sent with a
/// secret missing.
pub fn resolve_for_request(
    project: &Path,
    scope: &Scope,
    dir: &str,
    request_variables: &[Variable],
) -> Result<RequestScope> {
    let root = collection::load_root(project)?.request;
    let folders = collection::folder_defaults(project, dir)?;
    let vars = merge(layers(project, scope, &root.variables, &folders, request_variables, Need::Values)?);
    let inherited = std::iter::once(root).chain(folders.into_iter().map(|(_, defaults)| defaults)).collect();
    Ok(RequestScope { inherited, vars })
}

/// Where each variable a request in `dir` sees comes from. The same layers
/// as [`resolve_for_request`], so the two always agree, but secrets are
/// read by name only, so this works while they are locked.
pub fn sources_for_request(
    project: &Path,
    scope: &Scope,
    dir: &str,
    request_variables: &[Variable],
) -> Result<BTreeMap<String, VarSource>> {
    let root = collection::load_root(project)?.request;
    let folders = collection::folder_defaults(project, dir)?;
    let mut sources = BTreeMap::new();
    for (source, env) in layers(project, scope, &root.variables, &folders, request_variables, Need::Names)? {
        for name in env.into_keys() {
            sources.insert(name, source.clone());
        }
    }
    Ok(sources)
}

fn merge(layers: Vec<(VarSource, Env)>) -> Env {
    layers.into_iter().flat_map(|(_, env)| env).collect()
}

/// Every layer, lowest precedence first: the collection's variables, the
/// named env, the folders' (outermost first), the request's, secrets; then
/// with no critter the captured values, or with one the critter's
/// variables, its secrets and its context. A step's overrides come last.
///
/// A folder or request can override an environment but local secrets
/// still win, and with a critter the global captured values are left out,
/// so values never leak from one persona to another.
fn layers(
    project: &Path,
    scope: &Scope,
    collection_variables: &[Variable],
    folders: &[(String, RequestDefaults)],
    request_variables: &[Variable],
    need: Need,
) -> Result<Vec<(VarSource, Env)>> {
    let secret_layer = |file: SecretFile| -> Result<Env> {
        match need {
            Need::Values => secrets::read(project, &file),
            Need::Names => Ok(secrets::names(project, &file)?.into_iter().map(|name| (name, String::new())).collect()),
        }
    };
    let mut layers = vec![(VarSource::Collection, active_values(collection_variables))];
    if let Some(name) = &scope.env {
        layers.push((VarSource::Environment { name: name.clone() }, load(project, &EnvFile::Named(name.clone()))?));
    }
    for (dir, folder) in folders {
        layers.push((VarSource::Folder { dir: dir.clone() }, active_values(&folder.variables)));
    }
    layers.push((VarSource::Request, active_values(request_variables)));
    layers.push((VarSource::Secret, secret_layer(SecretFile::Project)?));
    match &scope.critter {
        None => layers.push((VarSource::Captured, load(project, &EnvFile::Captured)?)),
        Some(id) => {
            layers.push((VarSource::Critter, load(project, &EnvFile::Critter(id.clone()))?));
            layers.push((VarSource::CritterSecret, secret_layer(SecretFile::Critter(id.clone()))?));
            for (name, entry) in load_context(project, id)? {
                let source = VarSource::CritterContext { from: entry.from, at: entry.at };
                layers.push((source, Env::from([(name, entry.value)])));
            }
        }
    }
    if !scope.overrides.is_empty() {
        layers.push((VarSource::Step, scope.overrides.clone()));
    }
    Ok(layers)
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
        let active = Scope::active(p);
        assert_eq!(active, Scope::env(p, None));
        assert_eq!(resolve_for_request(p, &active, "f", &[]).unwrap().vars["c"], "folder");
        let request = [Variable::new("c", "request"), Variable::new("d", "request")];
        let scope = resolve_for_request(p, &active, "f", &request[..1]).unwrap();
        assert_eq!((scope.inherited.len(), scope.vars["c"].as_str()), (2, "request"));
        assert_eq!(resolve_for_request(p, &active, "f", &request).unwrap().vars["d"], "secret");
        assert_eq!(
            sources_for_request(p, &active, "f", &[]).unwrap().keys().collect::<Vec<_>>(),
            ["a", "b", "c", "d", "e"]
        );

        let sources = sources_for_request(p, &active, "f", &request).unwrap();
        let kinds: Vec<_> = ["a", "b", "c", "d", "e"].map(|k| sources[k].clone()).into();
        assert_eq!(
            kinds,
            [
                VarSource::Collection,
                VarSource::Environment { name: "dev".into() },
                VarSource::Request,
                VarSource::Secret,
                VarSource::Captured
            ]
        );
    }

    /// A project with every non-critter layer set, and critter Bob.
    fn project_with_bob() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        save(p, &EnvFile::Base, &env(&[("base", "base"), ("v", "base")])).unwrap();
        save(p, &EnvFile::Named("dev".into()), &env(&[("v", "dev")])).unwrap();
        set_active(p, Some("dev".into())).unwrap();
        save(p, &EnvFile::Secrets, &env(&[("s", "secret"), ("v", "secret")])).unwrap();
        merge_captured(p, &env(&[("leak", "global"), ("v", "captured")])).unwrap();
        crate::critter::create(p, "bob", &crate::critter::Critter::default()).unwrap();
        dir
    }

    #[test]
    fn critter_layers_override_in_order() {
        let dir = project_with_bob();
        let p = dir.path();
        let mut scope = Scope::active(p).with_critter(Some("bob".into()));
        let vars = |scope: &Scope| resolve_for_request(p, scope, "", &[]).unwrap().vars;
        assert_eq!(vars(&scope)["v"], "secret");
        assert!(!vars(&scope).contains_key("leak"), "global captured values are left out");

        save(p, &EnvFile::Critter("bob".into()), &env(&[("v", "critter"), ("c", "critter")])).unwrap();
        assert_eq!(vars(&scope)["v"], "critter");
        save(p, &EnvFile::CritterSecrets("bob".into()), &env(&[("v", "critter-secret")])).unwrap();
        assert_eq!(vars(&scope)["v"], "critter-secret");
        let from = crate::ProjectPath::parse("auth/login.yml").unwrap();
        crate::env::merge_context(p, "bob", &env(&[("v", "context")]), &from).unwrap();
        assert_eq!(vars(&scope)["v"], "context");
        scope.overrides = env(&[("v", "step")]);
        assert_eq!(vars(&scope)["v"], "step");
        assert_eq!(load(p, &EnvFile::CritterContext("bob".into())).unwrap(), env(&[("v", "context")]));

        let sources = sources_for_request(p, &scope, "", &[]).unwrap();
        let resolved = vars(&scope);
        assert_eq!(sources.keys().collect::<Vec<_>>(), resolved.keys().collect::<Vec<_>>());
        assert_eq!(
            (&sources["c"], &sources["v"], &sources["s"]),
            (&VarSource::Critter, &VarSource::Step, &VarSource::Secret)
        );
        scope.overrides.clear();
        let sources = sources_for_request(p, &scope, "", &[]).unwrap();
        assert!(matches!(&sources["v"], VarSource::CritterContext { from, .. } if from == "auth/login.yml"));

        assert!(matches!(
            resolve_for_request(p, &Scope::default().with_critter(Some("nobody".into())), "", &[]),
            Err(Error::CritterNotFound(_))
        ));
    }

    #[test]
    fn no_critter_resolves_exactly_the_old_layers() {
        let dir = project_with_bob();
        let p = dir.path();
        let vars = resolve_for_request(p, &Scope::active(p), "", &[]).unwrap().vars;
        assert_eq!(vars, env(&[("base", "base"), ("leak", "global"), ("s", "secret"), ("v", "captured")]));
        assert_eq!(resolve(p, None).unwrap(), vars);
    }

    #[test]
    fn critter_variables_save_in_place() {
        let dir = project_with_bob();
        let p = dir.path();
        let file = layout::critter_file(p, "bob");
        fs::write(&file, "name: Bob # me\nvariables:\n  - name: KEEP\n    secret: true\n  - name: a\n    value: '1'\n")
            .unwrap();
        save(p, &EnvFile::Critter("bob".into()), &env(&[("a", "2")])).unwrap();
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "name: Bob # me\nvariables:\n  - name: KEEP\n    secret: true\n  - name: a\n    value: '2'\n"
        );
        assert!(load(p, &EnvFile::CritterSecrets("../x".into())).is_err());
    }

    #[test]
    fn env_files_serialise_as_the_ui_names_them() {
        let json = |file: EnvFile| serde_json::to_string(&file).unwrap();
        assert_eq!(json(EnvFile::Base), r#"{"kind":"base"}"#);
        assert_eq!(json(EnvFile::Named("dev".into())), r#"{"kind":"named","name":"dev"}"#);
        assert_eq!(json(EnvFile::CritterSecrets("bob".into())), r#"{"kind":"critterSecrets","name":"bob"}"#);
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
