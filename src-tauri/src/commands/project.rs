//! Opening a project and listing its requests.

use std::path::{Path, PathBuf};

use cactus_core::scenario::{self, ScenarioEntry};
use cactus_core::{Result, recent, secrets};
use tauri::{AppHandle, Manager, State};

use crate::state::AppState;
use crate::watcher::ProjectWatcher;

/// List the project's scenarios, remember it as recent, start its enabled
/// schedules and start watching it. Switching projects locks the previous
/// one's secrets.
#[tauri::command(async)]
#[specta::specta]
pub fn open_project(app: AppHandle, state: State<'_, AppState>, project: PathBuf) -> Result<Vec<ScenarioEntry>> {
    let entries = scenario::list(&project)?;
    if state.project().as_deref() != Some(project.as_path()) {
        secrets::lock_all();
    }
    state.set_project(&project);
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

fn watch(app: &AppHandle, state: &AppState, project: &Path) {
    state.replace_watcher(ProjectWatcher::start(app, project));
}
