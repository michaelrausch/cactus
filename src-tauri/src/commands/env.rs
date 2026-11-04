use std::path::PathBuf;

use cactus_core::env::{self, Env, EnvFile};
use cactus_core::{Result, ScenarioPath, collection};

#[tauri::command(async)]
#[specta::specta]
pub fn list_envs(project: PathBuf) -> Result<Vec<String>> {
    env::list_named(&project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn active_env(project: PathBuf) -> Option<String> {
    env::active(&project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn set_active_env(project: PathBuf, name: Option<String>) -> Result<()> {
    env::set_active(&project, name)
}

#[tauri::command(async)]
#[specta::specta]
pub fn load_env(project: PathBuf, file: EnvFile) -> Result<Env> {
    env::load(&project, &file)
}

#[tauri::command(async)]
#[specta::specta]
pub fn save_env(project: PathBuf, file: EnvFile, env: Env) -> Result<()> {
    env::save(&project, &file, &env)
}

/// The names of every variable a request at `path` can use with the active
/// env, for highlighting and completing `{{name}}`. Values stay out of the UI.
#[tauri::command(async)]
#[specta::specta]
pub fn variable_names(project: PathBuf, path: ScenarioPath) -> Result<Vec<String>> {
    let inherited = collection::defaults_chain(&project, path.dir())?;
    let vars = env::resolve_for_request(&project, None, &inherited, &[])?;
    Ok(vars.into_keys().collect())
}
