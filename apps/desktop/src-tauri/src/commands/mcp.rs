//! The MCP server's settings.

use cactus_core::{Error, Result};
use tauri::{AppHandle, State};

use crate::mcp::{McpStatus, new_token};
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn mcp_status(state: State<'_, AppState>) -> Result<McpStatus> {
    Ok(state.mcp.status().await)
}

/// Save the settings and restart the server to match.
#[tauri::command]
#[specta::specta]
pub async fn set_mcp_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
    allow_edits: bool,
    port: u16,
) -> Result<McpStatus> {
    crate::commands::license::require_license()?;
    if port < 1024 {
        return Err(Error::InvalidPort(port));
    }
    state
        .mcp
        .update(&app, |settings| {
            settings.enabled = enabled;
            settings.allow_edits = allow_edits;
            settings.port = port;
        })
        .await
}

/// A new token; agents using the old one are turned away.
#[tauri::command]
#[specta::specta]
pub async fn regenerate_mcp_token(app: AppHandle, state: State<'_, AppState>) -> Result<McpStatus> {
    state.mcp.update(&app, |settings| settings.token = new_token()).await
}
