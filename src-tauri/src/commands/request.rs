use std::path::PathBuf;

use cactus_core::http::Response;
use cactus_core::runner::{self, RunResult};
use cactus_core::scenario::Scenario;
use cactus_core::{Result, ScenarioPath};
use tauri::{AppHandle, State};
use tauri_specta::Event;

use crate::events::RunProgress;
use crate::state::AppState;

/// Send the scenario as edited (it may not be saved yet). Resolves to
/// `None` when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn send_request(
    state: State<'_, AppState>,
    project: PathBuf,
    path: ScenarioPath,
    scenario: Scenario,
) -> Result<Option<Response>> {
    let send = runner::send_and_record(&state.client, &project, &path, &scenario, None);
    state.run_cancellable(send).await.transpose()
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
    let run = runner::run(&state.client, &project, &dir, None, report);
    state.run_cancellable(run).await.transpose()
}

#[tauri::command]
#[specta::specta]
pub fn cancel_request(state: State<'_, AppState>) {
    state.cancel_current();
}
