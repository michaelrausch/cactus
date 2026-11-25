//! Checking for a newer release on the website.

use cactus_core::{Error, Result, update};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::state::AppState;

/// The website's latest version, if it is newer than this build.
#[tauri::command]
#[specta::specta]
pub async fn check_for_update(state: State<'_, AppState>) -> Result<Option<String>> {
    update::check(&state.client, env!("CARGO_PKG_VERSION")).await
}

/// Open the website's downloads in the browser.
#[tauri::command]
#[specta::specta]
pub fn open_website(app: AppHandle) -> Result<()> {
    app.opener().open_url(update::DOWNLOAD_PAGE, None::<&str>).map_err(|e| Error::Browser(e.to_string()))
}
