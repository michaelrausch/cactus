use std::path::PathBuf;

use cactus_core::Result;
use cactus_core::critter::{self, Interval, validate_critter_id, validate_flow_id};
use cactus_core::monitor::{self, RunRecord};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, State};

use crate::scheduler::{self, split_key};
use crate::state::AppState;

/// Every schedule enabled on this machine, for the Scheduled list.
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
    /// `None` when the flow has no trigger or can't be read.
    pub every: Option<Interval>,
    pub env: Option<String>,
    /// Unix epoch millis of the next run; `None` when nothing is running.
    #[specta(type = Option<specta_typescript::Number>)]
    pub next_at: Option<u64>,
    /// The most recent run.
    pub last: Option<RunRecord>,
}

/// Turn a flow's schedule on or off on this machine.
#[tauri::command(async)]
#[specta::specta]
pub fn set_schedule_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    project: PathBuf,
    critter: String,
    flow: String,
    enabled: bool,
) -> Result<()> {
    critter::set_schedule_enabled(&project, &critter, &flow, enabled)?;
    state.scheduler.reload(&app, &project);
    Ok(())
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
    let schedules = critter::enabled_schedules(&project)
        .iter()
        .filter_map(|key| {
            let (critter, flow) = split_key(key)?;
            let trigger = critter::load_flow(&project, critter, flow).ok().and_then(|f| f.trigger);
            Some(ScheduleState {
                critter: critter.to_string(),
                flow: flow.to_string(),
                every: trigger.as_ref().map(|t| t.every),
                env: trigger.and_then(|t| t.env),
                next_at: state.scheduler.next_at(key),
                last: monitor::load_log(&project, critter, flow).pop(),
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
/// does: recorded, compared and alerted on.
#[tauri::command]
#[specta::specta]
pub async fn run_scheduled_now(app: AppHandle, project: PathBuf, critter: String, flow: String) -> Result<RunRecord> {
    validate_critter_id(&critter)?;
    validate_flow_id(&flow)?;
    Ok(scheduler::run_once(&app, &project, &critter, &flow).await)
}
