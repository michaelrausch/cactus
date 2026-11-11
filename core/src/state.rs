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
    /// The critter requests are sent as.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critter: Option<String>,
    /// Scheduled flows enabled on this machine, as `"<critter>/<flow>"`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub schedules: Vec<String>,
}

pub fn load(project: &Path) -> WorkspaceState {
    read_json_lenient(&layout::state_file(project))
}

/// Read, change and write the state in one step.
pub fn update(project: &Path, change: impl FnOnce(&mut WorkspaceState)) -> Result<()> {
    let mut state = load(project);
    change(&mut state);
    save(project, &state)
}

pub fn save(project: &Path, state: &WorkspaceState) -> Result<()> {
    layout::ensure_workspace(project)?;
    write_json(&layout::state_file(project), state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_state_files_load() {
        let state: WorkspaceState = serde_json::from_str(r#"{"env":"dev"}"#).unwrap();
        assert_eq!((state.env.as_deref(), state.critter.as_deref(), state.schedules.len()), (Some("dev"), None, 0));
        assert_eq!(serde_json::to_string(&state).unwrap(), r#"{"env":"dev"}"#);
    }
}
