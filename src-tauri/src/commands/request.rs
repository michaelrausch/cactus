//! Interactive sends: one request, or a folder. Starting one cancels the last.

use std::path::PathBuf;

use cactus_core::env::Scope;
use cactus_core::http::Response;
use cactus_core::oauth2::{self, TokenStatus};
use cactus_core::runner::{self, RunResult, SendOptions};
use cactus_core::scenario::Scenario;
use cactus_core::{Error, ProjectPath, Result};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::events::{ContextChanged, RunProgress, notify};
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
        notify(&RunProgress(result.clone()), &app);
    };
    let scope = Scope::active(&project);
    let run = runner::run(&state.client, &project, &dir, &scope, report);
    let result = state.run_cancellable(run).await.transpose();
    context_may_have_changed(&app, &scope);
    result
}

/// Sends as a critter may capture into its context, or log it in again.
fn context_may_have_changed(app: &AppHandle, scope: &Scope) {
    if let Some(critter) = &scope.critter {
        notify(&ContextChanged { critter: critter.clone() }, app);
    }
}

#[tauri::command]
#[specta::specta]
pub fn cancel_request(state: State<'_, AppState>) {
    state.cancel_current();
}

/// The cached OAuth2 token of the request (as edited) for the active env
/// and critter.
#[tauri::command(async)]
#[specta::specta]
pub fn oauth2_token_status(project: PathBuf, path: ProjectPath, scenario: Scenario) -> Result<TokenStatus> {
    oauth2::token_status(&project, &Scope::active(&project), &path, &scenario)
}

/// Get a new OAuth2 token for the request. The authorization code grant
/// opens the authorization page in the browser and waits for it to come
/// back. Resolves to `None` when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn get_oauth2_token(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    path: ProjectPath,
    scenario: Scenario,
) -> Result<Option<TokenStatus>> {
    let scope = Scope::active(&project);
    let open = |url: &str| {
        app.opener().open_url(url, None::<&str>).map_err(|e| Error::OAuth2(format!("couldn't open the browser: {e}")))
    };
    let get = oauth2::get_token(&state.client, &project, &scope, &path, &scenario, open);
    state.run_cancellable(get).await.transpose()
}

#[tauri::command(async)]
#[specta::specta]
pub fn clear_oauth2_token(project: PathBuf, path: ProjectPath, scenario: Scenario) -> Result<()> {
    oauth2::clear_token(&project, &Scope::active(&project), &path, &scenario)
}
