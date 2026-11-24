use std::path::PathBuf;

use cactus_core::Result;
use cactus_core::env::{self, Env, EnvFile};

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
