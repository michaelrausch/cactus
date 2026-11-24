use std::path::PathBuf;

use cactus_core::env::{self, Env, EnvFile};
use cactus_core::{ProjectPath, Result};

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

#[tauri::command(async)]
#[specta::specta]
pub fn rename_env(project: PathBuf, from: String, to: String) -> Result<()> {
    env::rename_named(&project, &from, &to)
}

#[tauri::command(async)]
#[specta::specta]
pub fn delete_env(project: PathBuf, name: String) -> Result<()> {
    env::delete_named(&project, &name)
}

/// The names of every variable a request at `path` can use with the active
/// env, for highlighting and completing `{{name}}`.
#[tauri::command(async)]
#[specta::specta]
pub fn variable_names(project: PathBuf, path: ProjectPath) -> Result<Vec<String>> {
    env::names_in_scope(&project, path.dir())
}
