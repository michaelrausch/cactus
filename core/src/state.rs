//! Per-machine UI state for a project, in `.cactus/state.json`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::storage::{read_json_lenient, write_json};
use crate::{Result, layout};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceState {
    /// The active named environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
}

pub fn load(project: &Path) -> WorkspaceState {
    read_json_lenient(&layout::state_file(project))
}

pub fn save(project: &Path, state: &WorkspaceState) -> Result<()> {
    layout::ensure_workspace(project)?;
    write_json(&layout::state_file(project), state)
}
