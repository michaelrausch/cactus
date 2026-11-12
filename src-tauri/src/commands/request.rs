use std::path::PathBuf;

use cactus_core::env::Scope;
use cactus_core::http::Response;
use cactus_core::runner::{self, RunResult, SendOptions};
use cactus_core::scenario::Scenario;
use cactus_core::{ProjectPath, Result};
use tauri::{AppHandle, State};
use tauri_specta::Event;

use crate::events::{ContextChanged, RunProgress};
use crate::state::AppState;

/// Send the scenario as edited (it may not be saved yet), as the active
/// env and critter. Resolves to `None` when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn send_request(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    path: ProjectPath,
    scenario: Scenario,
) -> Result<Option<Response>> {
    state.touch_secrets();
    let scope = Scope::active(&project);
    let send = runner::send_as(&state.client, &project, &path, &scenario, &scope, SendOptions::INTERACTIVE);
    let result = state.run_cancellable(send).await.transpose();
    context_may_have_changed(&app, &scope);
    result
}

/// Run every scenario under `dir`, emitting [`RunProgress`] per scenario.
/// Resolves to `None` when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn run_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    dir: String,
) -> Result<Option<Vec<RunResult>>> {
    let report = |result: &RunResult| {
        let _ = RunProgress(result.clone()).emit(&app);
    };
    state.touch_secrets();
    let scope = Scope::active(&project);
    let run = runner::run(&state.client, &project, &dir, &scope, report);
    let result = state.run_cancellable(run).await.transpose();
    context_may_have_changed(&app, &scope);
    result
}

/// Sends as a critter may capture into its context, or log it in again.
fn context_may_have_changed(app: &AppHandle, scope: &Scope) {
    if let Some(critter) = &scope.critter {
        let _ = ContextChanged { critter: critter.clone() }.emit(app);
    }
}

#[tauri::command]
#[specta::specta]
pub fn cancel_request(state: State<'_, AppState>) {
    state.cancel_current();
}
