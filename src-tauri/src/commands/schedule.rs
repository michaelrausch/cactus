use std::path::PathBuf;

use cactus_core::Result;
use cactus_core::critter::{self, Interval, validate_critter_id, validate_flow_id};
use cactus_core::monitor::{self, RunRecord};
use cactus_core::runner;
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, State};

use crate::scheduler::{self, split_key};
use crate::state::AppState;

/// Every scheduled flow and how it's doing.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleStates {
    /// Whether every schedule is paused.
    pub paused: bool,
    pub schedules: Vec<ScheduleState>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleState {
    pub critter: String,
    pub flow: String,
    /// `None` when the flow has no schedule or can't be read.
    pub every: Option<Interval>,
    pub env: Option<String>,
    /// Unix epoch millis of the next run; `None` when nothing is running.
    #[specta(type = Option<specta_typescript::Number>)]
    pub next_at: Option<u64>,
    /// The most recent run.
    pub last: Option<RunRecord>,
    /// Why the flow isn't running at its ticks, if something holds it.
    pub waiting: Option<ScheduleWaiting>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ScheduleWaiting {
    /// Its secrets are locked; it runs again once they're unlocked.
    SecretsLocked,
}

/// Pause or resume every schedule; nothing runs while paused.
#[tauri::command]
#[specta::specta]
pub fn set_schedules_paused(state: State<'_, AppState>, paused: bool) {
    state.scheduler.set_paused(paused);
}

#[tauri::command(async)]
#[specta::specta]
pub fn schedule_states(state: State<'_, AppState>, project: PathBuf) -> ScheduleStates {
    let schedules = critter::flow_keys(&project)
        .iter()
        .filter_map(|key| {
            let (critter, flow) = split_key(key)?;
            let definition = critter::load_flow(&project, critter, flow).ok();
            let schedule = definition.as_ref().and_then(|f| f.schedule());
            // A flow that can't be read is listed while its task still runs.
            let next_at = state.scheduler.next_at(key);
            if schedule.is_none() && next_at.is_none() {
                return None;
            }
            Some(ScheduleState {
                critter: critter.to_string(),
                flow: flow.to_string(),
                every: schedule.map(|s| s.every),
                env: schedule.and_then(|s| s.env.clone()),
                next_at,
                last: monitor::load_log(&project, critter, flow).pop(),
                waiting: scheduler::waiting_for_secrets(&project, critter).then_some(ScheduleWaiting::SecretsLocked),
            })
        })
        .collect();
    ScheduleStates { paused: state.scheduler.paused(), schedules }
}

/// The flow's recent runs, oldest first.
#[tauri::command(async)]
#[specta::specta]
pub fn load_run_log(project: PathBuf, critter: String, flow: String) -> Vec<RunRecord> {
    monitor::load_log(&project, &critter, &flow)
}

/// Run a scheduled flow once now, outside its timer, the same way the timer
/// does: recorded, compared and alerted on. While its secrets are locked
/// it's refused rather than recorded as a failing run.
#[tauri::command]
#[specta::specta]
pub async fn run_scheduled_now(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    critter: String,
    flow: String,
) -> Result<RunRecord> {
    validate_critter_id(&critter)?;
    validate_flow_id(&flow)?;
    runner::ensure_unlocked_for(&project, Some(&critter))?;
    state.touch_secrets();
    Ok(scheduler::run_once(&app, &project, &critter, &flow).await)
}
