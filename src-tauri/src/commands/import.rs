use std::path::PathBuf;

use cactus_core::Result;
use cactus_core::import::{self, ImportSummary};

#[tauri::command(async)]
#[specta::specta]
pub fn import_file(project: PathBuf, file: PathBuf) -> Result<ImportSummary> {
    import::import(&project, &file)
}
