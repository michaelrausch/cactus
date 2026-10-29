use std::path::PathBuf;

use cactus_core::history::{self, HistoryEntry};
use cactus_core::scenario::{self, Scenario};
use cactus_core::{Result, ScenarioPath};

#[tauri::command(async)]
#[specta::specta]
pub fn load_scenario(project: PathBuf, path: ScenarioPath) -> Result<Scenario> {
    scenario::load(&project, &path)
}

#[tauri::command(async)]
#[specta::specta]
pub fn save_scenario(project: PathBuf, path: ScenarioPath, scenario: Scenario) -> Result<()> {
    scenario::save(&project, &path, &scenario)
}

#[tauri::command(async)]
#[specta::specta]
pub fn create_scenario(project: PathBuf, dir: String, title: String) -> Result<ScenarioPath> {
    scenario::create_blank(&project, &dir, &title)
}

#[tauri::command(async)]
#[specta::specta]
pub fn duplicate_scenario(project: PathBuf, path: ScenarioPath) -> Result<ScenarioPath> {
    scenario::duplicate(&project, &path)
}

#[tauri::command(async)]
#[specta::specta]
pub fn delete_scenario(project: PathBuf, path: ScenarioPath) -> Result<()> {
    scenario::delete(&project, &path)
}

#[tauri::command(async)]
#[specta::specta]
pub fn rename_scenario(project: PathBuf, from: ScenarioPath, to: ScenarioPath) -> Result<()> {
    scenario::rename(&project, &from, &to)
}

#[tauri::command(async)]
#[specta::specta]
pub fn load_history(project: PathBuf, path: ScenarioPath) -> Vec<HistoryEntry> {
    history::load(&project, &path)
}
