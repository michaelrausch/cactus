//! The cactus desktop app: Tauri commands over `cactus_core`.

mod bindings;
mod commands;
mod events;
mod state;
mod watcher;

use tauri::Manager;

use crate::state::AppState;

const RECENT_PROJECTS_FILE: &str = "recent.json";

/// # Panics
/// If the Tauri runtime fails to start.
pub fn run() {
    let bindings = bindings::builder();
    #[cfg(debug_assertions)]
    bindings::export_typescript(&bindings);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(bindings.invoke_handler())
        .setup(move |app| {
            bindings.mount_events(app);
            let recent_projects_file = app.path().app_config_dir()?.join(RECENT_PROJECTS_FILE);
            app.manage(AppState::new(recent_projects_file));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running cactus");
}
