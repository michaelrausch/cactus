//! Runs enabled flows on their timers while the app is open, and raises
//! alerts when they start failing, recover or a watched value changes.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cactus_core::critter::{self, Interval};
use cactus_core::monitor::{self, RunRecord};
use cactus_core::reqwest;
use cactus_core::runner::{self, SendOptions};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::time::{Instant, MissedTickBehavior, interval_at};

use crate::events::{ContextChanged, MonitorAlert, ScheduledRunFinished};
use crate::state::AppState;

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
    running: HashMap<String, Task>,
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
        let current: HashMap<String, TaskSpec> =
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
            let task = self.spawn(app, project, &key, spec);
            tasks.running.insert(key, task);
        }
    }

    pub fn stop(&self) {
        let mut tasks = lock(&self.tasks);
        stop_all(&mut tasks.running);
        tasks.project = None;
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// When the task for `key` next runs, if one is running.
    pub fn next_at(&self, key: &str) -> Option<u64> {
        lock(&self.tasks).running.get(key).map(|task| task.next_at.load(Ordering::SeqCst))
    }

    fn spawn(&self, app: &AppHandle, project: &Path, key: &str, spec: TaskSpec) -> Task {
        let period = Duration::from_secs(u64::from(spec.every.seconds()));
        let next_at = Arc::new(AtomicU64::new(now_millis() + millis(period)));
        let (app, project, paused, next) =
            (app.clone(), project.to_path_buf(), Arc::clone(&self.paused), Arc::clone(&next_at));
        let (critter, flow) = split_key(key).map_or_default(|(c, f)| (c.to_string(), f.to_string()));
        let handle = tauri::async_runtime::spawn(async move {
            let mut tick = interval_at(Instant::now() + period, period);
            // After sleep or a long run, carry on from now rather than catching up.
            tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                next.store(now_millis() + millis(period), Ordering::SeqCst);
                if paused.load(Ordering::SeqCst) || waiting_for_secrets(&project, &critter) {
                    continue;
                }
                // Awaited, so a flow can't overlap itself.
                run_once(&app, &project, &critter, &flow).await;
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
    let definition = critter::load_flow(project, critter, flow);
    let rule = definition.as_ref().ok().and_then(|f| f.alert.clone()).unwrap_or_default();
    let env = definition.as_ref().ok().and_then(|f| f.schedule()?.env.clone());
    let started_at = now_millis();
    let run = runner::run_flow(&client, project, critter, flow, env.clone(), SendOptions::BACKGROUND, |_| {}).await;
    let record = match run {
        Ok(run) => monitor::record(&run),
        Err(error) => RunRecord::errored(started_at, env, error.to_string()),
    };
    let previous = monitor::load_log(project, critter, flow).pop();
    let events = monitor::compare(previous.as_ref(), &record, &rule);
    // The log is a convenience; failing to write it shouldn't stop the schedule.
    let _ = monitor::append(project, critter, flow, record.clone());
    let (critter, flow) = (critter.to_string(), flow.to_string());
    let _ = ScheduledRunFinished { critter: critter.clone(), flow: flow.clone(), record: record.clone() }.emit(app);
    for event in events {
        let alert = MonitorAlert { critter: critter.clone(), flow: flow.clone(), event, at: record.at };
        let _ = alert.emit(app);
    }
    let _ = ContextChanged { critter }.emit(app);
    record
}

/// Whether the flow's secrets are locked. Such a tick runs nothing, logs
/// nothing and alerts nothing: a locked vault isn't the API failing, and
/// recording it as a failure would page you for walking away. Tasks pick
/// up at their next tick once unlocked, with no catch-up runs.
pub fn waiting_for_secrets(project: &Path, critter: &str) -> bool {
    runner::ensure_unlocked_for(project, Some(critter)).is_err()
}

/// The tasks the project asks for: each flow with a schedule. A flow that
/// can't be read keeps the task it had, so it goes on running and reports
/// itself as failing.
fn wanted(project: &Path, current: &HashMap<String, TaskSpec>) -> BTreeMap<String, TaskSpec> {
    let mut wanted = BTreeMap::new();
    for key in critter::flow_keys(project) {
        let Some((critter, flow)) = split_key(&key) else { continue };
        let spec = match critter::load_flow(project, critter, flow) {
            Ok(flow) => flow.schedule().map(|schedule| TaskSpec { every: schedule.every, env: schedule.env.clone() }),
            Err(_) if critter::exists(project, critter) => current.get(&key).cloned(),
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
    pub stop: Vec<String>,
    pub start: Vec<String>,
}

pub fn plan(current: &HashMap<String, TaskSpec>, wanted: &BTreeMap<String, TaskSpec>) -> Changes {
    let mut changes = Changes::default();
    let mut current_keys: Vec<&String> = current.keys().collect();
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

/// `"bob/health"` → `("bob", "health")`.
pub fn split_key(key: &str) -> Option<(&str, &str)> {
    let (critter, flow) = key.split_once('/')?;
    (!critter.is_empty() && !flow.is_empty()).then_some((critter, flow))
}

fn stop_all(running: &mut HashMap<String, Task>) {
    for (_, task) in running.drain() {
        task.handle.abort();
    }
}

fn now_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, millis)
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

    #[test]
    fn plan_restarts_only_what_changed() {
        let current = HashMap::from([
            ("bob/same".to_string(), spec("5m", None)),
            ("bob/slower".to_string(), spec("5m", None)),
            ("bob/gone".to_string(), spec("1m", None)),
            ("bob/env".to_string(), spec("1m", Some("dev"))),
        ]);
        let wanted = BTreeMap::from([
            ("bob/same".to_string(), spec("5m", None)),
            ("bob/slower".to_string(), spec("10m", None)),
            ("bob/env".to_string(), spec("1m", Some("staging"))),
            ("bob/new".to_string(), spec("30s", None)),
        ]);
        let changes = plan(&current, &wanted);
        assert_eq!(changes.stop, ["bob/env", "bob/gone", "bob/slower"]);
        assert_eq!(changes.start, ["bob/env", "bob/new", "bob/slower"]);
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

        let current = HashMap::from([("bob/broken".to_string(), spec("1m", None))]);
        let wanted = wanted(p, &current);
        // `gone` can't be read and had no task, so there's nothing to keep.
        assert_eq!(wanted.keys().collect::<Vec<_>>(), ["bob/broken", "bob/health"]);
        assert_eq!(wanted["bob/broken"], spec("1m", None), "an unreadable flow keeps its task");
        assert_eq!(wanted["bob/health"], spec("5m", None));
    }

    #[test]
    fn keys_split_into_critter_and_flow() {
        assert_eq!(split_key("bob/health"), Some(("bob", "health")));
        assert_eq!(split_key("bob/"), None);
        assert_eq!(split_key("bob"), None);
    }
}
