use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use cactus_core::critter::{self, Flow, FlowParse, FlowSummary};
use cactus_core::env::{self, Scope, VarSource};
use cactus_core::runner::{self, FlowRun, FlowStepResult, MatrixCell, SendOptions};
use cactus_core::scenario::Scenario;
use cactus_core::{ProjectPath, Result};
use tauri::{AppHandle, State};
use tauri_specta::Event;

use crate::events::{ContextChanged, FlowProgress, MatrixProgress};
use crate::state::AppState;

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
    critter::save_flow(&project, &critter, &flow, &definition)?;
    state.scheduler.reload(&app, &project);
    Ok(())
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
    critter::create_flow(&project, &critter, &flow, &definition)?;
    state.scheduler.reload(&app, &project);
    Ok(())
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
    critter::rename_flow(&project, &critter, &from, &to)?;
    state.scheduler.reload(&app, &project);
    Ok(())
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
    critter::delete_flow(&project, &critter, &flow)?;
    state.scheduler.reload(&app, &project);
    Ok(())
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
    critter::save_flow_text(&project, &critter, &flow, &text)?;
    state.scheduler.reload(&app, &project);
    Ok(())
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

/// Run a flow, emitting [`FlowProgress`] per step. `env` overrides the
/// active one. Resolves to `None` when cancelled.
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
    let report = |step: &FlowStepResult| {
        let _ = FlowProgress { critter: critter.clone(), flow: flow.clone(), step: step.clone() }.emit(&app);
    };
    let run = runner::run_flow(&state.client, &project, &critter, &flow, env, SendOptions::INTERACTIVE, report);
    let result = state.run_cancellable(run).await.transpose();
    let _ = ContextChanged { critter: critter.clone() }.emit(&app);
    result
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
        let _ = MatrixProgress(cell.clone()).emit(&app);
    };
    let run = runner::run_matrix(&state.client, &project, &path, &scenario, report);
    let result = state.run_cancellable(run).await.transpose();
    for critter in sent_as {
        let _ = ContextChanged { critter }.emit(&app);
    }
    result
}
