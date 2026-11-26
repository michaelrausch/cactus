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
    open(&app, update::DOWNLOAD_PAGE)
}

/// Open where to buy a Pro license key in the browser.
#[tauri::command]
#[specta::specta]
pub fn open_pro_page(app: AppHandle) -> Result<()> {
    open(&app, update::PRO_PAGE)
}

fn open(app: &AppHandle, url: &str) -> Result<()> {
    app.opener().open_url(url, None::<&str>).map_err(|e| Error::Browser(e.to_string()))
}
