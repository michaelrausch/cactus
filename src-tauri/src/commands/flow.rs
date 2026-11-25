//! Flows: files, the YAML editor, runs, and the critter matrix.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use cactus_core::critter::{self, Flow, FlowParse, FlowSummary};
use cactus_core::env::{self, Scope, VarSource};
use cactus_core::runner::{self, FlowEvent, FlowRun, MatrixCell, SendOptions};
use cactus_core::scenario::Scenario;
use cactus_core::{ProjectPath, Result};
use tauri::{AppHandle, State};

use crate::events::{ContextChanged, FlowProgress, FlowStepStarted, MatrixProgress, notify};
use crate::state::AppState;
use crate::tool_host::DesktopHost;

/// Make a change to a critter's flow files, then have the scheduler pick up
/// any trigger it changed. The file watcher would too, but only after its
/// debounce, and saving a schedule should show its next run straight away.
fn changing_flows(
    app: &AppHandle,
    state: &AppState,
    project: &Path,
    change: impl FnOnce() -> Result<()>,
) -> Result<()> {
    change()?;
    state.scheduler.reload(app, project);
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub fn list_flows(project: PathBuf, critter: String) -> Result<Vec<FlowSummary>> {
    critter::list_flows(&project, &critter)
}

#[tauri::command(async)]
#[specta::specta]
pub fn load_flow(project: PathBuf, critter: String, flow: String) -> Result<Flow> {
    critter::load_flow(&project, &critter, &flow)
}

/// Save by editing the flow file in place. A changed trigger takes effect
/// at once.
#[tauri::command(async)]
#[specta::specta]
pub fn save_flow(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    critter: String,
    flow: String,
    definition: Flow,
) -> Result<()> {
    changing_flows(&app, &state, &project, || critter::save_flow(&project, &critter, &flow, &definition))
}

#[tauri::command(async)]
#[specta::specta]
pub fn create_flow(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    critter: String,
    flow: String,
    definition: Flow,
) -> Result<()> {
    changing_flows(&app, &state, &project, || critter::create_flow(&project, &critter, &flow, &definition))
}

#[tauri::command(async)]
#[specta::specta]
pub fn rename_flow(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    critter: String,
    from: String,
    to: String,
) -> Result<()> {
    changing_flows(&app, &state, &project, || critter::rename_flow(&project, &critter, &from, &to))
}

/// Copy a flow, comments and all, as `to` named `name`.
#[tauri::command(async)]
#[specta::specta]
pub fn duplicate_flow(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    critter: String,
    from: String,
    to: String,
    name: String,
) -> Result<()> {
    changing_flows(&app, &state, &project, || critter::duplicate_flow(&project, &critter, &from, &to, &name))
}

#[tauri::command(async)]
#[specta::specta]
pub fn delete_flow(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    critter: String,
    flow: String,
) -> Result<()> {
    changing_flows(&app, &state, &project, || critter::delete_flow(&project, &critter, &flow))
}

#[tauri::command(async)]
#[specta::specta]
pub fn load_flow_text(project: PathBuf, critter: String, flow: String) -> Result<String> {
    critter::load_flow_text(&project, &critter, &flow)
}

/// Write the YAML editor's text as typed; text that isn't a valid flow is refused.
#[tauri::command(async)]
#[specta::specta]
pub fn save_flow_text(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    critter: String,
    flow: String,
    text: String,
) -> Result<()> {
    changing_flows(&app, &state, &project, || critter::save_flow_text(&project, &critter, &flow, &text))
}

#[tauri::command(async)]
#[specta::specta]
pub fn parse_flow(text: String) -> FlowParse {
    critter::parse_flow(&text)
}

/// `definition` as YAML, to show unsaved visual edits in the YAML editor.
#[tauri::command(async)]
#[specta::specta]
pub fn flow_to_yaml(definition: Flow) -> Result<String> {
    critter::flow_yaml(&definition)
}

/// Where each variable a flow step can use comes from, as `critter` in the
/// active env at the project root: what a flow's steps see before `with`.
#[tauri::command(async)]
#[specta::specta]
pub fn flow_variable_sources(project: PathBuf, critter: String) -> Result<BTreeMap<String, VarSource>> {
    let scope = Scope::env(&project, None).with_critter(Some(critter));
    env::sources_for_request(&project, &scope, "", &[])
}

/// Run a flow, emitting [`FlowStepStarted`] as each step sends and
/// [`FlowProgress`] as it finishes. `env` overrides the active one.
/// Resolves to `None` when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn run_flow(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    critter: String,
    flow: String,
    env: Option<String>,
) -> Result<Option<FlowRun>> {
    let report = |event: FlowEvent<'_>| {
        let (critter, flow) = (critter.clone(), flow.clone());
        match event {
            FlowEvent::Started(start) => notify(&FlowStepStarted { critter, flow, start: start.clone() }, &app),
            FlowEvent::Finished(step) => notify(&FlowProgress { critter, flow, step: step.clone() }, &app),
        }
    };
    let host = DesktopHost { app: app.clone(), critter: critter.clone(), flow: flow.clone(), interactive: true };
    let run = runner::run_flow(&state.client, &project, &critter, &flow, env, SendOptions::INTERACTIVE, &host, report);
    let result = state.run_cancellable(run).await.transpose();
    notify(&ContextChanged { critter }, &app);
    result
}

/// Answer a running flow's question ([`FlowAsked`](crate::events::FlowAsked)):
/// the values, or `None` to stop the run.
#[tauri::command]
#[specta::specta]
pub fn answer_flow_ask(state: State<'_, AppState>, id: u32, values: Option<BTreeMap<String, String>>) {
    state.asks.answer(id, values);
}

/// Send the request as edited as no critter and as each critter,
/// emitting [`MatrixProgress`] per row. Resolves to `None` when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn run_matrix(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    path: ProjectPath,
    scenario: Scenario,
) -> Result<Option<Vec<MatrixCell>>> {
    let mut sent_as = BTreeSet::new();
    let report = |cell: &MatrixCell| {
        if let (Some(critter), Some(_)) = (&cell.critter, &cell.response) {
            sent_as.insert(critter.clone());
        }
        notify(&MatrixProgress(cell.clone()), &app);
    };
    let run = runner::run_matrix(&state.client, &project, &path, &scenario, report);
    let result = state.run_cancellable(run).await.transpose();
    for critter in sent_as {
        notify(&ContextChanged { critter }, &app);
    }
    result
}
