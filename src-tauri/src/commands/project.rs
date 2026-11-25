//! Opening a project and listing its requests.

use std::path::{Path, PathBuf};

use cactus_core::scenario::{self, ScenarioEntry};
use cactus_core::secrets::{self, TotpSetup, Unlocked, VaultStatus};
use cactus_core::{Result, recent};
use tauri::{AppHandle, Manager, State};

use crate::state::AppState;
use crate::watcher::ProjectWatcher;

/// List the project's scenarios, remember it as recent, start its enabled
/// schedules and start watching it.
#[tauri::command(async)]
#[specta::specta]
pub fn open_project(app: AppHandle, state: State<'_, AppState>, project: PathBuf) -> Result<Vec<ScenarioEntry>> {
    let entries = scenario::list(&project)?;
    recent::push(&state.recent_projects_file, &project.to_string_lossy())?;
    state.scheduler.reload(&app, &project);
    // Registering a watch per folder can be slow on big trees; the list needn't wait for it.
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        watch(&app, &state, &project);
    });
    Ok(entries)
}

/// Re-list, and re-watch so folders created since opening are covered.
#[tauri::command(async)]
#[specta::specta]
pub fn list_scenarios(app: AppHandle, state: State<'_, AppState>, project: PathBuf) -> Result<Vec<ScenarioEntry>> {
    let entries = scenario::list(&project)?;
    watch(&app, &state, &project);
    Ok(entries)
}

#[tauri::command]
#[specta::specta]
pub fn recent_projects(state: State<'_, AppState>) -> Vec<String> {
    recent::load(&state.recent_projects_file)
}

#[tauri::command]
#[specta::specta]
pub fn vault_status(project: PathBuf) -> VaultStatus {
    secrets::status(&project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn unlock_vault(project: PathBuf, password: String, code: Option<String>) -> Result<Unlocked> {
    secrets::unlock(&project, &password, code.as_deref())
}

#[tauri::command]
#[specta::specta]
pub fn totp_enabled(project: PathBuf) -> bool {
    secrets::totp_enabled(&project)
}

#[tauri::command]
#[specta::specta]
pub fn start_totp(project: PathBuf) -> Result<TotpSetup> {
    secrets::start_totp(&project)
}

#[tauri::command]
#[specta::specta]
pub fn enable_totp(project: PathBuf, secret: String, code: String) -> Result<()> {
    secrets::enable_totp(&project, &secret, &code)
}

#[tauri::command]
#[specta::specta]
pub fn disable_totp(project: PathBuf, code: String) -> Result<()> {
    secrets::disable_totp(&project, &code)
}

/// Delete the project's secrets, context and password.
#[tauri::command(async)]
#[specta::specta]
pub fn wipe_vault(project: PathBuf) -> Result<()> {
    secrets::wipe(&project)
}

/// Set, change or (with no `next`) remove the project's password.
#[tauri::command(async)]
#[specta::specta]
pub fn set_vault_password(project: PathBuf, current: Option<String>, next: Option<String>) -> Result<()> {
    secrets::set_password(&project, current.as_deref(), next.as_deref())
}

fn watch(app: &AppHandle, state: &AppState, project: &Path) {
    state.replace_watcher(ProjectWatcher::start(app, project));
}
