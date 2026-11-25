//! Request files and their history.

use std::path::PathBuf;

use cactus_core::history::{self, HistoryEntry};
use cactus_core::scenario::{self, Scenario};
use cactus_core::{ProjectPath, Result};

#[tauri::command(async)]
#[specta::specta]
pub fn load_scenario(project: PathBuf, path: ProjectPath) -> Result<Scenario> {
    scenario::load(&project, &path)
}

#[tauri::command(async)]
#[specta::specta]
pub fn save_scenario(project: PathBuf, path: ProjectPath, scenario: Scenario) -> Result<()> {
    scenario::save(&project, &path, &scenario)
}

#[tauri::command(async)]
#[specta::specta]
pub fn create_scenario(
    project: PathBuf,
    dir: String,
    title: String,
    method: String,
    url: String,
) -> Result<ProjectPath> {
    scenario::create_request(&project, &dir, &title, &method, &url)
}

/// Set a request's title, and name its file after it. Returns where it ends up.
#[tauri::command(async)]
#[specta::specta]
pub fn retitle_scenario(project: PathBuf, path: ProjectPath, title: String) -> Result<ProjectPath> {
    scenario::retitle(&project, &path, &title)
}

#[tauri::command(async)]
#[specta::specta]
pub fn create_folder(project: PathBuf, dir: String) -> Result<()> {
    scenario::create_folder(&project, &dir)
}

/// Request folders, empty ones included.
#[tauri::command(async)]
#[specta::specta]
pub fn list_folders(project: PathBuf) -> Vec<String> {
    scenario::list_folders(&project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn duplicate_scenario(project: PathBuf, path: ProjectPath) -> Result<ProjectPath> {
    scenario::duplicate(&project, &path)
}

#[tauri::command(async)]
#[specta::specta]
pub fn delete_scenario(project: PathBuf, path: ProjectPath) -> Result<()> {
    scenario::delete(&project, &path)
}

#[tauri::command(async)]
#[specta::specta]
pub fn rename_scenario(project: PathBuf, from: ProjectPath, to: ProjectPath) -> Result<()> {
    scenario::rename(&project, &from, &to)
}

#[tauri::command(async)]
#[specta::specta]
pub fn load_history(project: PathBuf, path: ProjectPath) -> Vec<HistoryEntry> {
    history::load(&project, &path)
}
