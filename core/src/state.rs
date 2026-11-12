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
    /// Set while secret encryption is on; see `crate::secrets`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secrets: Option<SecretSettings>,
}

/// Secret encryption on this machine. Settings only: no key material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretSettings {
    /// Lock after this many idle minutes; `null` means never.
    #[serde(default = "default_auto_lock")]
    pub auto_lock_minutes: Option<u32>,
}

/// What a project locks after when encryption is turned on, in minutes.
pub const DEFAULT_AUTO_LOCK_MINUTES: u32 = 15;

#[allow(clippy::unnecessary_wraps)]
fn default_auto_lock() -> Option<u32> {
    Some(DEFAULT_AUTO_LOCK_MINUTES)
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
        assert_eq!((state.env.as_deref(), state.critter.as_deref()), (Some("dev"), None));
        assert_eq!(serde_json::to_string(&state).unwrap(), r#"{"env":"dev"}"#);
        // The per-machine schedule list cactus used to keep is ignored.
        let old: WorkspaceState = serde_json::from_str(r#"{"env":"dev","schedules":["bob/health"]}"#).unwrap();
        assert_eq!(serde_json::to_string(&old).unwrap(), r#"{"env":"dev"}"#);
    }
}
