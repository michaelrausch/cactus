use std::path::PathBuf;

use cactus_core::Result;
use cactus_core::critter::{self, Critter, CritterSummary, Species};
use cactus_core::env::{self, Context};
use tauri::{AppHandle, State};
use tauri_specta::Event;

use crate::events::ContextChanged;
use crate::state::AppState;

#[tauri::command(async)]
#[specta::specta]
pub fn list_critters(project: PathBuf) -> Result<Vec<CritterSummary>> {
    critter::list(&project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn load_critter(project: PathBuf, id: String) -> Result<Critter> {
    critter::load(&project, &id)
}

#[tauri::command(async)]
#[specta::specta]
pub fn save_critter(project: PathBuf, id: String, critter: Critter) -> Result<()> {
    critter::save(&project, &id, &critter)
}

#[tauri::command(async)]
#[specta::specta]
pub fn create_critter(project: PathBuf, id: String, critter: Critter) -> Result<()> {
    critter::create(&project, &id, &critter)
}

/// Rename a critter; its enabled schedules follow.
#[tauri::command(async)]
#[specta::specta]
pub fn rename_critter(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    from: String,
    to: String,
) -> Result<()> {
    critter::rename(&project, &from, &to)?;
    state.scheduler.reload(&app, &project);
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub fn duplicate_critter(project: PathBuf, from: String, to: String, name: String, species: Species) -> Result<()> {
    critter::duplicate(&project, &from, &to, &name, species)
}

#[tauri::command(async)]
#[specta::specta]
pub fn delete_critter(app: AppHandle, state: State<'_, AppState>, project: PathBuf, id: String) -> Result<()> {
    critter::delete(&project, &id)?;
    state.scheduler.reload(&app, &project);
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub fn active_critter(project: PathBuf) -> Option<String> {
    critter::active(&project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn set_active_critter(project: PathBuf, id: Option<String>) -> Result<()> {
    critter::set_active(&project, id)
}

#[tauri::command(async)]
#[specta::specta]
pub fn load_context(project: PathBuf, id: String) -> Result<Context> {
    env::load_context(&project, &id)
}

/// Put back entries [`clear_context`] removed (Undo).
#[tauri::command(async)]
#[specta::specta]
pub fn restore_context(app: AppHandle, project: PathBuf, id: String, entries: Context) -> Result<()> {
    env::restore_context(&project, &id, &entries)?;
    let _ = ContextChanged { critter: id }.emit(&app);
    Ok(())
}

/// Remove the named entries (all when `names` is `None`) and return them,
/// so the UI can offer Undo.
#[tauri::command(async)]
#[specta::specta]
pub fn clear_context(app: AppHandle, project: PathBuf, id: String, names: Option<Vec<String>>) -> Result<Context> {
    let removed = env::clear_context(&project, &id, names.as_deref())?;
    let _ = ContextChanged { critter: id }.emit(&app);
    Ok(removed)
}
