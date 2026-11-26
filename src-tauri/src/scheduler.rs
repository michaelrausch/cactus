//! Runs enabled flows on their timers while the app is open, and raises
//! alerts when they start failing, recover or a watched value changes.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use cactus_core::best_effort;
use cactus_core::critter::{self, FlowKey, Interval};
use cactus_core::monitor::{self, RunRecord};
use cactus_core::now_unix_millis;
use cactus_core::reqwest;
use cactus_core::runner::{self, SendOptions};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};
use tokio::time::{Instant, MissedTickBehavior, interval_at};

use crate::events::{ContextChanged, MonitorAlert, ScheduledRunFinished, notify};
use crate::state::AppState;
use crate::tool_host::DesktopHost;

/// What a running task was started with; a change restarts it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpec {
    pub every: Interval,
    pub env: Option<String>,
}

struct Task {
    spec: TaskSpec,
    /// Unix epoch millis of the next tick.
    next_at: Arc<AtomicU64>,
    handle: JoinHandle<()>,
}

#[derive(Default)]
struct Tasks {
    project: Option<PathBuf>,
    running: HashMap<FlowKey, Task>,
}

/// Owned by [`AppState`]. Scheduled runs don't go through
/// `run_cancellable`, so they never cancel, and are never cancelled by,
/// interactive sends.
#[derive(Default)]
pub struct Scheduler {
    tasks: Mutex<Tasks>,
    paused: Arc<AtomicBool>,
}

impl Scheduler {
    /// Make the running tasks match the project's enabled schedules. Tasks
    /// whose spec is unchanged keep their timers; opening another project
    /// stops every task of the previous one.
    pub fn reload(&self, app: &AppHandle, project: &Path) {
        let mut tasks = lock(&self.tasks);
        if tasks.project.as_deref() != Some(project) {
            stop_all(&mut tasks.running);
            tasks.project = Some(project.to_path_buf());
        }
        let current: HashMap<FlowKey, TaskSpec> =
            tasks.running.iter().map(|(key, task)| (key.clone(), task.spec.clone())).collect();
        let wanted = wanted(project, &current);
        let changes = plan(&current, &wanted);
        for key in &changes.stop {
            if let Some(task) = tasks.running.remove(key) {
                task.handle.abort();
            }
        }
        for key in changes.start {
            let spec = wanted[&key].clone();
            let task = self.spawn(app, project, key.clone(), spec);
            tasks.running.insert(key, task);
        }
    }

    pub fn stop(&self) {
        let mut tasks = lock(&self.tasks);
        stop_all(&mut tasks.running);
        tasks.project = None;
    }

    // The flag and the next-run time are read on their own; nothing else is
    // published through them, so no ordering is needed.
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// When the task for `key` next runs, if one is running.
    pub fn next_at(&self, key: &FlowKey) -> Option<u64> {
        lock(&self.tasks).running.get(key).map(|task| task.next_at.load(Ordering::Relaxed))
    }

    fn spawn(&self, app: &AppHandle, project: &Path, key: FlowKey, spec: TaskSpec) -> Task {
        let period = Duration::from_secs(u64::from(spec.every.seconds()));
        let next_at = Arc::new(AtomicU64::new(now_unix_millis() + millis(period)));
        let (app, project, paused, next) =
            (app.clone(), project.to_path_buf(), Arc::clone(&self.paused), Arc::clone(&next_at));
        let handle = tauri::async_runtime::spawn(async move {
            let mut tick = interval_at(Instant::now() + period, period);
            // After sleep or a long run, carry on from now rather than catching up.
            tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                next.store(now_unix_millis() + millis(period), Ordering::Relaxed);
                if paused.load(Ordering::Relaxed) {
                    continue;
                }
                // Awaited, so a flow can't overlap itself.
                run_once(&app, &project, &key.critter, &key.flow).await;
            }
        });
        Task { spec, next_at, handle }
    }
}

/// Run a scheduled flow once, in its trigger's env: record it, compare it
/// with the previous run, append it to the log and tell the UI. A run that
/// can't start (say the flow no longer parses) is a failing run, which
/// alerts once rather than crashing the task.
pub async fn run_once(app: &AppHandle, project: &Path, critter: &str, flow: &str) -> RunRecord {
    let client: reqwest::Client = app.state::<AppState>().client.clone();
    // Loaded once, so the rule and env read here are those of the version that runs.
    let definition = critter::load_flow(project, critter, flow);
    // A flow that can't be read fails the run, and alerts with the default rule.
    let rule = definition.as_ref().ok().and_then(|f| f.alert.clone()).unwrap_or_default();
    let env = definition.as_ref().ok().and_then(|f| f.schedule()?.env.clone());
    let started_at = now_unix_millis();
    let run = match &definition {
        Ok(definition) => {
            let options = SendOptions::BACKGROUND;
            // No one to ask, but a `notify` step still notifies.
            let host = DesktopHost::new(app.clone(), critter.into(), flow.into(), false);
            runner::run_flow_definition(&client, project, critter, definition, env.clone(), options, &host, |_| {})
                .await
                .map_err(|error| error.to_string())
        }
        Err(error) => Err(error.to_string()),
    };
    let record = match run {
        Ok(run) => monitor::record(&run),
        Err(error) => RunRecord::errored(started_at, env, error),
    };
    let events = monitor::compare(&monitor::load_log(project, critter, flow), &record, &rule);
    // A log that can't be written fails no run, but the next run then
    // compares against an older one, so say so.
    best_effort::log("write the run log", monitor::append(project, critter, flow, record.clone()));
    announce(app, critter, flow, &record, events);
    record
}

/// Tell the UI about a finished scheduled run and any alerts it raised.
fn announce(app: &AppHandle, critter: &str, flow: &str, record: &RunRecord, events: Vec<monitor::AlertEvent>) {
    let (critter, flow) = (critter.to_string(), flow.to_string());
    notify(&ScheduledRunFinished { critter: critter.clone(), flow: flow.clone(), record: record.clone() }, app);
    for event in events {
        notify(&MonitorAlert { critter: critter.clone(), flow: flow.clone(), event, at: record.at }, app);
    }
    notify(&ContextChanged { critter }, app);
}

/// The tasks the project asks for: each flow with a schedule. A flow that
/// can't be read keeps the task it had, so it goes on running and reports
/// itself as failing.
fn wanted(project: &Path, current: &HashMap<FlowKey, TaskSpec>) -> BTreeMap<FlowKey, TaskSpec> {
    let mut wanted = BTreeMap::new();
    for key in critter::flow_keys(project) {
        let spec = match critter::load_flow(project, &key.critter, &key.flow) {
            Ok(flow) => flow.schedule().map(|schedule| TaskSpec { every: schedule.every, env: schedule.env.clone() }),
            Err(_) if critter::exists(project, &key.critter) => current.get(&key).cloned(),
            Err(_) => None,
        };
        if let Some(spec) = spec {
            wanted.insert(key, spec);
        }
    }
    wanted
}

/// Which tasks to stop and start to get from `current` to `wanted`:
/// unchanged ones are left alone, changed ones restarted.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Changes {
    pub stop: Vec<FlowKey>,
    pub start: Vec<FlowKey>,
}

pub fn plan(current: &HashMap<FlowKey, TaskSpec>, wanted: &BTreeMap<FlowKey, TaskSpec>) -> Changes {
    let mut changes = Changes::default();
    let mut current_keys: Vec<&FlowKey> = current.keys().collect();
    current_keys.sort();
    for key in current_keys {
        if wanted.get(key) != current.get(key) {
            changes.stop.push(key.clone());
        }
    }
    for (key, spec) in wanted {
        if current.get(key) != Some(spec) {
            changes.start.push(key.clone());
        }
    }
    changes
}

fn stop_all(running: &mut HashMap<FlowKey, Task>) {
    for (_, task) in running.drain() {
        task.handle.abort();
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// The tasks stay consistent even if a holder panicked, so poisoning is ignored.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use cactus_core::critter::{Critter, Flow, Schedule, Trigger};

    use super::*;

    fn spec(every: &str, env: Option<&str>) -> TaskSpec {
        TaskSpec { every: Interval::parse(every).unwrap(), env: env.map(str::to_string) }
    }

    fn bob(flow: &str) -> FlowKey {
        FlowKey::new("bob", flow)
    }

    #[test]
    fn plan_restarts_only_what_changed() {
        let current = HashMap::from([
            (bob("same"), spec("5m", None)),
            (bob("slower"), spec("5m", None)),
            (bob("gone"), spec("1m", None)),
            (bob("env"), spec("1m", Some("dev"))),
        ]);
        let wanted = BTreeMap::from([
            (bob("same"), spec("5m", None)),
            (bob("slower"), spec("10m", None)),
            (bob("env"), spec("1m", Some("staging"))),
            (bob("new"), spec("30s", None)),
        ]);
        let changes = plan(&current, &wanted);
        assert_eq!(changes.stop, [bob("env"), bob("gone"), bob("slower")]);
        assert_eq!(changes.start, [bob("env"), bob("new"), bob("slower")]);
        assert_eq!(plan(&current, &current.clone().into_iter().collect()), Changes::default());
    }

    #[test]
    fn wanted_reads_flows_with_schedules() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        critter::create(p, "bob", &Critter::default()).unwrap();
        let schedule = Schedule { every: Interval::parse("5m").unwrap(), env: None };
        let triggered = Flow { triggers: vec![Trigger::Schedule(schedule)], ..Flow::default() };
        critter::create_flow(p, "bob", "health", &triggered).unwrap();
        assert_eq!(critter::load_flow(p, "bob", "health").unwrap(), triggered);
        critter::create_flow(p, "bob", "manual", &Flow::default()).unwrap();
        critter::create_flow(p, "bob", "broken", &triggered).unwrap();
        critter::create_flow(p, "bob", "gone", &triggered).unwrap();
        fs::write(p.join("critters/bob/flows/broken.yml"), "steps: [").unwrap();
        fs::write(p.join("critters/bob/flows/gone.yml"), "steps: [").unwrap();

        let current = HashMap::from([(bob("broken"), spec("1m", None))]);
        let wanted = wanted(p, &current);
        // `gone` can't be read and had no task, so there's nothing to keep.
        assert_eq!(wanted.keys().collect::<Vec<_>>(), [&bob("broken"), &bob("health")]);
        assert_eq!(wanted[&bob("broken")], spec("1m", None), "an unreadable flow keeps its task");
        assert_eq!(wanted[&bob("health")], spec("5m", None));
    }

    #[test]
    fn keys_read_as_critter_slash_flow() {
        assert_eq!(bob("health").to_string(), "bob/health");
    }
}
