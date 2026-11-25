//! Events the backend pushes to the UI.

use cactus_core::best_effort;
use cactus_core::monitor::{AlertEvent, RunRecord};
use cactus_core::runner::{FlowStepResult, FlowStepStart, MatrixCell, RunResult};
use cactus_core::tools::Ask;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

/// Push `event` to the UI. Work goes on if that fails, but the failure is logged.
pub fn notify<E: Event + Serialize + Clone, R: tauri::Runtime, H: tauri::Emitter<R> + tauri::Manager<R>>(
    event: &E,
    app: &H,
) {
    best_effort::log("notify the UI", event.emit(app));
}

/// One step of an interactive flow run is sending.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FlowStepStarted {
    pub critter: String,
    pub flow: String,
    pub start: FlowStepStart,
}

/// One step of an interactive flow run finished.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FlowProgress {
    pub critter: String,
    pub flow: String,
    pub step: FlowStepResult,
}

/// A flow's tool step asks the person running it something; answer with
/// `answer_flow_ask(id, …)`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FlowAsked {
    pub id: u32,
    pub critter: String,
    pub flow: String,
    pub ask: Ask,
}

/// The question `id` no longer needs an answer: its run was cancelled.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FlowAskClosed {
    pub id: u32,
}

/// A flow's `display` step shows a message, without waiting for it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FlowDisplayed {
    pub critter: String,
    pub flow: String,
    pub title: String,
    pub text: String,
}

/// A flow's `notify` step ran.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FlowNotified {
    pub critter: String,
    pub flow: String,
    pub title: String,
    pub body: String,
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
