//! Secret encryption. Passwords cross IPC once, as `String`, and are moved
//! into `SecretString` at once; they're never logged or put in errors.

use std::path::{Path, PathBuf};

use cactus_core::env::EnvFile;
use cactus_core::secrets::{self, SecretFile, SecretString, SecretsInfo, UnlockReport};
use cactus_core::{Error, Result};
use tauri::{AppHandle, State};
use tauri_specta::Event;

use crate::events::SecretsStatusChanged;
use crate::state::AppState;

/// Tell the UI the project's lock state may have changed.
pub fn status_changed(app: &AppHandle, project: &Path) {
    if let Ok(info) = secrets::info(project) {
        let _ = SecretsStatusChanged { status: info.status }.emit(app);
    }
}

#[tauri::command(async)]
#[specta::specta]
pub fn secrets_status(project: PathBuf) -> Result<SecretsInfo> {
    secrets::info(&project)
}

#[tauri::command]
#[specta::specta]
pub async fn enable_secrets_encryption(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    password: String,
    auto_lock_minutes: Option<u32>,
) -> Result<()> {
    let password = SecretString::new(password);
    let path = project.clone();
    blocking(move || secrets::enable(&path, &password, auto_lock_minutes)).await?;
    state.touch_secrets();
    status_changed(&app, &project);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn disable_secrets_encryption(app: AppHandle, project: PathBuf, password: String) -> Result<()> {
    let password = SecretString::new(password);
    let path = project.clone();
    blocking(move || secrets::disable(&path, &password)).await?;
    status_changed(&app, &project);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn change_secrets_password(project: PathBuf, current: String, next: String) -> Result<()> {
    let (current, new) = (SecretString::new(current), SecretString::new(next));
    blocking(move || secrets::change_password(&project, &current, &new)).await
}

/// Argon2id takes about half a second, so it runs off the async workers.
#[tauri::command]
#[specta::specta]
pub async fn unlock_secrets(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    password: String,
) -> Result<UnlockReport> {
    let password = SecretString::new(password);
    let path = project.clone();
    let report = blocking(move || secrets::unlock(&path, &password)).await?;
    state.touch_secrets();
    status_changed(&app, &project);
    Ok(report)
}

#[tauri::command]
#[specta::specta]
pub fn lock_secrets(app: AppHandle, project: PathBuf) {
    secrets::lock(&project);
    status_changed(&app, &project);
}

/// For a forgotten password: sealed values are removed, plain ones kept.
#[tauri::command(async)]
#[specta::specta]
pub fn reset_secrets(app: AppHandle, project: PathBuf) -> Result<()> {
    secrets::reset(&project)?;
    status_changed(&app, &project);
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub fn set_auto_lock(project: PathBuf, minutes: Option<u32>) -> Result<()> {
    secrets::set_auto_lock(&project, minutes)
}

/// The names in a secrets layer, which work while locked.
#[tauri::command(async)]
#[specta::specta]
pub fn secret_names(project: PathBuf, file: EnvFile) -> Result<Vec<String>> {
    let file = match file {
        EnvFile::Secrets => SecretFile::Project,
        EnvFile::CritterSecrets(id) => SecretFile::Critter(id),
        _ => return Err(Error::InvalidPath("not a secrets layer".into())),
    };
    secrets::names(&project, &file)
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?
}
