//! Events the backend pushes to the UI.

use cactus_core::monitor::{AlertEvent, RunRecord};
use cactus_core::runner::{FlowStepResult, MatrixCell, RunResult};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

/// One step of an interactive flow run finished.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FlowProgress {
    pub critter: String,
    pub flow: String,
    pub step: FlowStepResult,
}

/// One row of a matrix run finished.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct MatrixProgress(pub MatrixCell);

/// A scheduled flow ran (on its timer or through "Run now").
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ScheduledRunFinished {
    pub critter: String,
    pub flow: String,
    pub record: RunRecord,
}

/// A scheduled flow started failing, recovered, or a watched value changed.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct MonitorAlert {
    pub critter: String,
    pub flow: String,
    pub event: AlertEvent,
    /// Unix epoch millis.
    #[specta(type = specta_typescript::Number)]
    pub at: u64,
}

/// A critter's captured context may have changed. Writes to `.cactus/` are
/// invisible to the watcher, so the UI needs this to stay current.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ContextChanged {
    pub critter: String,
}

/// Project-relative paths of files that changed on disk.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FsChanged(pub Vec<String>);

/// One scenario of a folder run finished.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct RunProgress(pub RunResult);
