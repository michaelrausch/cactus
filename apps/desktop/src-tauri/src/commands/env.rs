//! Environments, secret files, and where each `{{variable}}` comes from.

use std::collections::BTreeMap;
use std::path::PathBuf;

use cactus_core::env::{self, Env, EnvFile, Scope, VarSource};
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

/// Every variable a request at `path` can use as the active env and
/// critter, and where its value comes from, for highlighting, completing
/// and explaining `{{name}}`.
#[tauri::command(async)]
#[specta::specta]
pub fn variable_sources(project: PathBuf, path: ProjectPath) -> Result<BTreeMap<String, VarSource>> {
    env::sources_for_request(&project, &Scope::active(&project), path.dir(), &[])
}

/// Where each variable comes from in the active env with no critter, at
/// the project root: what a critter's own variables override.
#[tauri::command(async)]
#[specta::specta]
pub fn base_variable_sources(project: PathBuf) -> Result<BTreeMap<String, VarSource>> {
    env::sources_for_request(&project, &Scope::env(&project, None), "", &[])
}
