//! The cactus desktop app: Tauri commands over `cactus_core`.

mod bindings;
mod commands;
mod events;
mod mcp;
mod scheduler;
mod state;
mod tool_host;
mod watcher;

use tauri::{Manager, RunEvent};

use crate::state::AppState;

const RECENT_PROJECTS_FILE: &str = "recent.json";
const MCP_SETTINGS_FILE: &str = "mcp.json";

/// # Panics
/// If the Tauri runtime fails to start.
pub fn run() {
    let bindings = bindings::builder();
    #[cfg(debug_assertions)]
    bindings::export_typescript(&bindings);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        // Flows' `open` tool, from Rust only: the UI gets no opener permission.
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(bindings.invoke_handler())
        .setup(move |app| {
            bindings.mount_events(app);
            let config_dir = app.path().app_config_dir()?;
            app.manage(AppState::new(config_dir.join(RECENT_PROJECTS_FILE), config_dir.join(MCP_SETTINGS_FILE)));
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                handle.state::<AppState>().mcp.start(&handle).await;
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building cactus")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                let state = app.state::<AppState>();
                state.scheduler.stop();
                state.mcp.stop();
            }
        });
}
