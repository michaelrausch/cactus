//! Creating, listing and organising critters and their flows on disk.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Serialize;

use super::model::{Critter, Flow, Species, Trigger, validate_critter_id, validate_flow_id};
use super::secrets::missing_secrets;
use crate::num::saturating_u32;
use crate::storage::write_atomic;
use crate::{Error, Result, best_effort, collection, layout, state};

/// What the UI lists for a critter, in one call.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CritterSummary {
    pub id: String,
    pub name: String,
    pub species: Species,
    pub notes: String,
    pub login: Option<String>,
    pub flows: Vec<FlowSummary>,
    pub missing_secrets: Vec<String>,
    /// Why `critter.yml` couldn't be read; the rest is then defaults.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FlowSummary {
    pub id: String,
    pub name: String,
    pub steps: u32,
    pub triggers: Vec<Trigger>,
    /// Why the flow file couldn't be read.
    pub error: Option<String>,
}

/// The outcome of parsing flow text as it is typed. Invalid YAML while
/// typing is normal, so it is a value, not an error.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum FlowParse {
    Ok { flow: Flow },
    Invalid { message: String, line: Option<u32>, column: Option<u32> },
}

/// A flow by its critter and file name, shown as `bob/health`: how the
/// scheduler and the UI's schedule list name a flow.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FlowKey {
    pub critter: String,
    pub flow: String,
}

impl FlowKey {
    pub fn new(critter: impl Into<String>, flow: impl Into<String>) -> Self {
        Self { critter: critter.into(), flow: flow.into() }
    }
}

impl std::fmt::Display for FlowKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.critter, self.flow)
    }
}

/// Every critter in `critters/`, sorted by id. Folders without a
/// `critter.yml` or with an invalid id are left out.
pub fn list(project: &Path) -> Vec<CritterSummary> {
    let mut ids = child_names(&layout::critters_dir(project), Path::is_dir);
    ids.retain(|id| exists(project, id));
    ids.sort();
    ids.into_iter().map(|id| summarise(project, id)).collect()
}

fn summarise(project: &Path, id: String) -> CritterSummary {
    let (critter, mut error) = match load(project, &id) {
        Ok(critter) => (critter, None),
        Err(error) => (Critter::default(), Some(error.to_string())),
    };
    // Not being able to tell which secrets are missing is a problem to show,
    // not "none missing".
    let missing_secrets = missing_secrets(project, &id).unwrap_or_else(|failure| {
        error.get_or_insert_with(|| failure.to_string());
        Vec::new()
    });
    let name = if critter.name.is_empty() { id.clone() } else { critter.name.0 };
    CritterSummary {
        flows: flow_summaries(project, &id),
        missing_secrets,
        name,
        species: critter.species,
        notes: critter.notes.0,
        login: critter.login.map(|login| login.0),
        error,
        id,
    }
}

/// Names of the entries in `dir` that pass `keep`; none if it can't be read.
fn child_names(dir: &Path, keep: impl Fn(&Path) -> bool) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    entries
        .filter_map(std::result::Result::ok)
        .filter(|entry| keep(&entry.path()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

pub fn exists(project: &Path, id: &str) -> bool {
    validate_critter_id(id).is_ok() && layout::critter_file(project, id).is_file()
}

/// `Ok` when the critter exists; otherwise an invalid id is reported as
/// such, and a valid one as not found.
pub fn require(project: &Path, id: &str) -> Result<()> {
    if exists(project, id) {
        return Ok(());
    }
    validate_critter_id(id)?;
    Err(Error::CritterNotFound(id.into()))
}

/// Make the new folder `dir`, failing with [`Error::AlreadyExists`] if it is
/// already there. Creating it is the check, so two creators can't both pass.
fn create_new_dir(dir: &Path, what: String) -> Result<()> {
    if let Some(parent) = dir.parent() {
        fs::create_dir_all(parent).map_err(Error::file(parent))?;
    }
    fs::create_dir(dir).map_err(|e| match e.kind() {
        ErrorKind::AlreadyExists => Error::AlreadyExists(what),
        _ => Error::file(dir)(e),
    })
}

pub fn load(project: &Path, id: &str) -> Result<Critter> {
    validate_critter_id(id)?;
    collection::read_if_exists(&layout::critter_file(project, id))?.ok_or_else(|| Error::CritterNotFound(id.into()))
}

/// Save by editing `critter.yml` in place.
pub fn save(project: &Path, id: &str, critter: &Critter) -> Result<()> {
    validate_critter_id(id)?;
    let file = layout::critter_file(project, id);
    if !file.is_file() {
        return Err(Error::CritterNotFound(id.into()));
    }
    collection::write(&file, critter)
}

pub fn create(project: &Path, id: &str, critter: &Critter) -> Result<()> {
    validate_critter_id(id)?;
    let dir = layout::critter_dir(project, id);
    create_new_dir(&dir, format!("critter {id}"))?;
    undo_on_error(&dir, collection::write(&layout::critter_file(project, id), critter))
}

/// Remove the folder `dir` just created when what was put in it failed, so
/// a failed create or copy doesn't leave a critter that can't be read.
fn undo_on_error<T>(dir: &Path, result: Result<T>) -> Result<T> {
    if result.is_err() {
        best_effort::log("remove the half-made folder", fs::remove_dir_all(dir));
    }
    result
}

/// Move the critter's folder and its local files; the active critter and
/// its enabled schedules follow.
pub fn rename(project: &Path, from: &str, to: &str) -> Result<()> {
    validate_critter_id(from)?;
    validate_critter_id(to)?;
    if from == to {
        return Ok(());
    }
    require(project, from)?;
    let target = layout::critter_dir(project, to);
    if target.exists() {
        return Err(Error::AlreadyExists(format!("critter {to}")));
    }
    let source = layout::critter_dir(project, from);
    fs::rename(&source, &target).map_err(Error::file(&source))?;
    move_if_exists(&layout::critter_workspace_dir(project, from), &layout::critter_workspace_dir(project, to))?;
    state::update(project, |state| {
        if state.critter.as_deref() == Some(from) {
            state.critter = Some(to.into());
        }
    })
}

/// Copy `critter.yml` and the flows under a new id, name and species.
/// Secrets, context and run logs stay with the original.
pub fn duplicate(project: &Path, from: &str, to: &str, name: &str, species: Species) -> Result<()> {
    let mut critter = load(project, from)?;
    validate_critter_id(to)?;
    let target = layout::critter_dir(project, to);
    create_new_dir(&target, format!("critter {to}"))?;
    let copied = copy_critter_files(project, from, to).and_then(|()| {
        critter.name = name.into();
        critter.species = species;
        save(project, to, &critter)
    });
    undo_on_error(&target, copied)
}

fn copy_critter_files(project: &Path, from: &str, to: &str) -> Result<()> {
    copy_file(&layout::critter_file(project, from), &layout::critter_file(project, to))?;
    let flows = layout::flows_dir(project, from);
    let flow_names = child_names(&flows, Path::is_file);
    if flow_names.is_empty() {
        return Ok(());
    }
    let target_flows = layout::flows_dir(project, to);
    fs::create_dir_all(&target_flows).map_err(Error::file(&target_flows))?;
    for file_name in flow_names {
        copy_file(&flows.join(&file_name), &target_flows.join(&file_name))?;
    }
    Ok(())
}

/// Delete the critter's folder and local files, and forget it in the state.
pub fn delete(project: &Path, id: &str) -> Result<()> {
    validate_critter_id(id)?;
    let dir = layout::critter_dir(project, id);
    if !dir.exists() {
        return Err(Error::CritterNotFound(id.into()));
    }
    fs::remove_dir_all(&dir).map_err(Error::file(&dir))?;
    remove_dir_if_exists(&layout::critter_workspace_dir(project, id))?;
    state::update(project, |state| {
        if state.critter.as_deref() == Some(id) {
            state.critter = None;
        }
    })
}

/// The critter requests are sent as, if it still exists.
pub fn active(project: &Path) -> Option<String> {
    state::load(project).critter.filter(|id| exists(project, id))
}

pub fn set_active(project: &Path, id: Option<String>) -> Result<()> {
    if let Some(id) = &id {
        require(project, id)?;
    }
    state::update(project, |state| state.critter = id)
}

/// Every critter's flows, readable or not, for the scheduler to pick out
/// the ones with a schedule.
pub fn flow_keys(project: &Path) -> Vec<FlowKey> {
    list(project)
        .iter()
        .flat_map(|critter| critter.flows.iter().map(|flow| FlowKey::new(&critter.id, &flow.id)))
        .collect()
}

/// The critter's flows, sorted by id. A flow that doesn't parse is listed
/// with its error.
pub fn list_flows(project: &Path, id: &str) -> Result<Vec<FlowSummary>> {
    validate_critter_id(id)?;
    Ok(flow_summaries(project, id))
}

/// [`list_flows`] for an id that is already known to be valid.
fn flow_summaries(project: &Path, id: &str) -> Vec<FlowSummary> {
    let mut flows: Vec<String> = child_names(&layout::flows_dir(project, id), Path::is_file)
        .iter()
        .filter_map(|file_name| layout::flow_from_file_name(file_name))
        .filter(|flow| validate_flow_id(flow).is_ok())
        .map(str::to_string)
        .collect();
    flows.sort();
    flows
        .into_iter()
        .map(|flow_id| match load_flow(project, id, &flow_id) {
            Ok(flow) => FlowSummary {
                name: if flow.name.is_empty() { flow_id.clone() } else { flow.name.0 },
                steps: saturating_u32(flow.steps.len()),
                triggers: flow.triggers,
                error: None,
                id: flow_id,
            },
            Err(error) => FlowSummary {
                name: flow_id.clone(),
                steps: 0,
                triggers: Vec::new(),
                error: Some(error.to_string()),
                id: flow_id,
            },
        })
        .collect()
}

fn flow_path(project: &Path, id: &str, flow: &str) -> Result<std::path::PathBuf> {
    validate_critter_id(id)?;
    validate_flow_id(flow)?;
    Ok(layout::flow_file(project, id, flow))
}

fn flow_not_found(id: &str, flow: &str) -> Error {
    Error::FlowNotFound { critter: id.into(), flow: flow.into() }
}

/// An I/O failure on a flow's `file`: a missing file is the flow not found.
fn flow_io_error<'a>(file: &'a Path, id: &'a str, flow: &'a str) -> impl Fn(std::io::Error) -> Error + 'a {
    move |e| match e.kind() {
        ErrorKind::NotFound => flow_not_found(id, flow),
        _ => Error::file(file)(e),
    }
}

pub fn load_flow(project: &Path, id: &str, flow: &str) -> Result<Flow> {
    collection::read_if_exists(&flow_path(project, id, flow)?)?.ok_or_else(|| flow_not_found(id, flow))
}

/// Save by editing the flow file in place.
pub fn save_flow(project: &Path, id: &str, flow_id: &str, flow: &Flow) -> Result<()> {
    let file = flow_path(project, id, flow_id)?;
    if !file.is_file() {
        return Err(flow_not_found(id, flow_id));
    }
    collection::write(&file, flow)
}

pub fn create_flow(project: &Path, id: &str, flow_id: &str, flow: &Flow) -> Result<()> {
    let file = flow_path(project, id, flow_id)?;
    require(project, id)?;
    if file.exists() {
        return Err(Error::AlreadyExists(format!("flow {flow_id}")));
    }
    let dir = layout::flows_dir(project, id);
    fs::create_dir_all(&dir).map_err(Error::file(&dir))?;
    collection::write(&file, flow)
}

/// Copy a flow's file, comments and all, to a new file name and flow name.
/// Its run log stays with the original.
pub fn duplicate_flow(project: &Path, id: &str, from: &str, to: &str, name: &str) -> Result<()> {
    let text = load_flow_text(project, id, from)?;
    let target = flow_path(project, id, to)?;
    if target.exists() {
        return Err(Error::AlreadyExists(format!("flow {to}")));
    }
    save_flow_text(project, id, to, &text)?;
    let named = load_flow(project, id, to).and_then(|mut flow| {
        flow.name = name.into();
        save_flow(project, id, to, &flow)
    });
    if named.is_err() {
        best_effort::log("remove the half-made copy", fs::remove_file(&target));
    }
    named
}

/// Rename a flow file, with its run log and the critter's
/// `login` if it pointed at it.
pub fn rename_flow(project: &Path, id: &str, from: &str, to: &str) -> Result<()> {
    let source = flow_path(project, id, from)?;
    let target = flow_path(project, id, to)?;
    if from == to {
        return Ok(());
    }
    if !source.is_file() {
        return Err(flow_not_found(id, from));
    }
    if target.exists() {
        return Err(Error::AlreadyExists(format!("flow {to}")));
    }
    // Read before anything moves, so a critter that can't be read fails the
    // rename while it is still a no-op.
    let critter = load(project, id)?;
    fs::rename(&source, &target).map_err(Error::file(&source))?;
    let (log_from, log_to) = (layout::run_log_file(project, id, from), layout::run_log_file(project, id, to));
    let followed = move_if_exists(&log_from, &log_to).and_then(|()| point_login_at(project, id, critter, from, to));
    if followed.is_err() {
        best_effort::log("undo the flow rename", fs::rename(&target, &source));
        best_effort::log("undo the run log move", move_if_exists(&log_to, &log_from));
    }
    followed
}

/// Make the critter's `login` follow a renamed flow.
fn point_login_at(project: &Path, id: &str, mut critter: Critter, from: &str, to: &str) -> Result<()> {
    if critter.login.as_ref().is_some_and(|login| login == from) {
        critter.login = Some(to.into());
        save(project, id, &critter)?;
    }
    Ok(())
}

/// Delete a flow file and its run log.
pub fn delete_flow(project: &Path, id: &str, flow: &str) -> Result<()> {
    let file = flow_path(project, id, flow)?;
    fs::remove_file(&file).map_err(flow_io_error(&file, id, flow))?;
    let log = layout::run_log_file(project, id, flow);
    if let Err(e) = fs::remove_file(&log)
        && e.kind() != ErrorKind::NotFound
    {
        return Err(Error::file(&log)(e));
    }
    Ok(())
}

/// The flow file exactly as written, for the YAML editor.
pub fn load_flow_text(project: &Path, id: &str, flow: &str) -> Result<String> {
    let file = flow_path(project, id, flow)?;
    fs::read_to_string(&file).map_err(flow_io_error(&file, id, flow))
}

/// Write flow text verbatim, keeping comments and formatting as typed. Text
/// that doesn't parse as a flow is refused.
pub fn save_flow_text(project: &Path, id: &str, flow: &str, text: &str) -> Result<()> {
    let file = flow_path(project, id, flow)?;
    require(project, id)?;
    parse_flow_text(text)?;
    let dir = layout::flows_dir(project, id);
    fs::create_dir_all(&dir).map_err(Error::file(&dir))?;
    write_atomic(&file, text.as_bytes())
}

fn parse_flow_text(text: &str) -> std::result::Result<Flow, serde_yaml_ng::Error> {
    serde_yaml_ng::from_str(if text.trim().is_empty() { "{}" } else { text })
}

/// `flow` as a new file would be written, e.g. to show unsaved visual edits
/// in the YAML editor.
pub fn flow_yaml(flow: &Flow) -> Result<String> {
    Ok(collection::to_yaml(&serde_json::to_value(flow)?)?)
}

pub fn parse_flow(text: &str) -> FlowParse {
    match parse_flow_text(text) {
        Ok(flow) => FlowParse::Ok { flow },
        Err(error) => {
            let location = error.location();
            let position = |n: usize| u32::try_from(n).ok();
            FlowParse::Invalid {
                message: error.to_string(),
                line: location.as_ref().and_then(|l| position(l.line())),
                column: location.as_ref().and_then(|l| position(l.column())),
            }
        }
    }
}

fn copy_file(from: &Path, to: &Path) -> Result<()> {
    fs::copy(from, to).map(drop).map_err(Error::file(from))
}

fn move_if_exists(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(Error::file(parent))?;
    }
    fs::rename(from, to).map_err(Error::file(from))
}

fn remove_dir_if_exists(dir: &Path) -> Result<()> {
    match fs::remove_dir_all(dir) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(Error::file(dir)(e)),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::critter::FlowStep;
    use crate::env::{self, Env, EnvFile};

    fn save_secret(project: &Path, id: &str) {
        env::save(project, &EnvFile::CritterSecrets(id.into()), &Env::from([("k".into(), "v".into())])).unwrap();
    }

    fn bob() -> Critter {
        Critter { name: "Bob".into(), species: Species::Gecko, ..Critter::default() }
    }

    fn one_step(request: &str) -> Flow {
        Flow {
            name: "F".into(),
            steps: vec![FlowStep { request: request.into(), ..FlowStep::default() }],
            ..Flow::default()
        }
    }

    #[test]
    fn create_list_rename_delete() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        assert!(list(p).is_empty());
        create(p, "bob", &bob()).unwrap();
        assert!(matches!(create(p, "bob", &bob()), Err(Error::AlreadyExists(_))));
        assert!(matches!(create(p, "../x", &bob()), Err(Error::InvalidCritterName(_))));
        create_flow(p, "bob", "login", &one_step("auth/login.yml")).unwrap();
        fs::create_dir_all(p.join("critters/no-file")).unwrap();
        fs::write(p.join("critters/bob/flows/broken.yml"), "steps: [").unwrap();

        let critters = list(p);
        assert_eq!(critters.len(), 1);
        let summary = &critters[0];
        assert_eq!((summary.id.as_str(), summary.name.as_str(), summary.species), ("bob", "Bob", Species::Gecko));
        let flows: Vec<_> = summary.flows.iter().map(|f| (f.id.as_str(), f.steps, f.error.is_some())).collect();
        assert_eq!(flows, [("broken", 0, true), ("login", 1, false)]);

        let mut critter = load(p, "bob").unwrap();
        critter.login = Some("login".into());
        save(p, "bob", &critter).unwrap();
        set_active(p, Some("bob".into())).unwrap();
        assert_eq!(flow_keys(p), [FlowKey::new("bob", "broken"), FlowKey::new("bob", "login")]);
        save_secret(p, "bob");
        assert!(matches!(set_active(p, Some("nobody".into())), Err(Error::CritterNotFound(_))));

        rename(p, "bob", "robert").unwrap();
        assert!(!p.join("critters/bob").exists() && p.join("critters/robert/flows/login.yml").exists());
        assert_eq!(crate::secrets::names(p, &crate::secrets::SecretFile::Critter("robert".into())).unwrap(), ["k"]);
        assert_eq!(active(p).as_deref(), Some("robert"));

        rename_flow(p, "robert", "login", "sign-in").unwrap();
        assert_eq!(load(p, "robert").unwrap().login.unwrap(), "sign-in");
        assert_eq!(flow_keys(p), [FlowKey::new("robert", "broken"), FlowKey::new("robert", "sign-in")]);
        delete_flow(p, "robert", "sign-in").unwrap();
        assert_eq!(flow_keys(p), [FlowKey::new("robert", "broken")]);
        assert!(matches!(delete_flow(p, "robert", "sign-in"), Err(Error::FlowNotFound { .. })));

        delete(p, "robert").unwrap();
        assert!(!p.join("critters/robert").exists() && !layout::critter_workspace_dir(p, "robert").exists());
        assert_eq!(active(p), None);
    }

    #[test]
    fn duplicate_copies_files_but_not_local_state() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        create(p, "bob", &bob()).unwrap();
        fs::write(layout::critter_file(p, "bob"), "# Bob's notes\nname: Bob\nspecies: gecko\n").unwrap();
        create_flow(p, "bob", "login", &one_step("auth/login.yml")).unwrap();
        save_secret(p, "bob");

        duplicate(p, "bob", "alice", "Alice", Species::ElfOwl).unwrap();
        assert_eq!(
            fs::read_to_string(layout::critter_file(p, "alice")).unwrap(),
            "# Bob's notes\nname: Alice\nspecies: elf-owl\n"
        );
        assert_eq!(load_flow(p, "alice", "login").unwrap(), one_step("auth/login.yml"));
        assert!(!layout::critter_workspace_dir(p, "alice").exists());
        assert!(matches!(duplicate(p, "bob", "alice", "A", Species::Gecko), Err(Error::AlreadyExists(_))));
    }

    #[test]
    fn flow_text_round_trips_and_parse_reports_position() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        create(p, "bob", &bob()).unwrap();
        let text = "# health check\nname: H\nsteps:\n  - request: a.yml # first\n";
        save_flow_text(p, "bob", "h", text).unwrap();
        assert_eq!(load_flow_text(p, "bob", "h").unwrap(), text);
        assert!(save_flow_text(p, "bob", "h", "steps: [").is_err());
        assert_eq!(load_flow_text(p, "bob", "h").unwrap(), text);

        let mut flow = load_flow(p, "bob", "h").unwrap();
        flow.steps[0].expect = Some(200);
        save_flow(p, "bob", "h", &flow).unwrap();
        let saved = load_flow_text(p, "bob", "h").unwrap();
        assert!(saved.starts_with("# health check\n") && saved.contains("request: a.yml # first"), "{saved}");

        assert!(matches!(parse_flow(text), FlowParse::Ok { .. }));
        let emitted = flow_yaml(&flow).unwrap();
        assert!(emitted.starts_with("name: H\nsteps:\n  - request: a.yml\n    expect: 200\n"), "{emitted}");
        assert!(matches!(parse_flow(&emitted), FlowParse::Ok { flow: parsed } if parsed == flow));
        match parse_flow("name: x\nsteps:\n  - request: [a\n") {
            FlowParse::Invalid { line, column, .. } => assert!(line.is_some() && column.is_some()),
            FlowParse::Ok { .. } => panic!("expected invalid"),
        }
        match parse_flow("steps:\n  - expect: nope\n") {
            FlowParse::Invalid { line, .. } => assert_eq!(line, Some(2)),
            FlowParse::Ok { .. } => panic!("expected invalid"),
        }
    }

    #[test]
    fn a_duplicated_flow_keeps_its_comments_and_takes_the_new_name() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        create(p, "bob", &bob()).unwrap();
        save_flow_text(p, "bob", "h", "# health check\nname: H\nsteps:\n  - request: a.yml # first\n").unwrap();

        duplicate_flow(p, "bob", "h", "h-copy", "H copy").unwrap();
        let copy = load_flow_text(p, "bob", "h-copy").unwrap();
        assert!(copy.starts_with("# health check\n") && copy.contains("request: a.yml # first"), "{copy}");
        assert_eq!(load_flow(p, "bob", "h-copy").unwrap().name.as_str(), "H copy");
        assert_eq!(load_flow(p, "bob", "h").unwrap().name.as_str(), "H");

        assert!(matches!(duplicate_flow(p, "bob", "h", "h-copy", "again"), Err(Error::AlreadyExists(_))));
        assert!(matches!(duplicate_flow(p, "bob", "nope", "x", "x"), Err(Error::FlowNotFound { .. })));
    }

    #[test]
    fn require_tells_a_bad_id_from_a_missing_critter() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        create(p, "bob", &bob()).unwrap();
        assert!(require(p, "bob").is_ok());
        assert!(matches!(require(p, "nobody"), Err(Error::CritterNotFound(_))));
        assert!(matches!(require(p, "../x"), Err(Error::InvalidCritterName(_))));
    }
}
