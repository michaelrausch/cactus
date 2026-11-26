//! Scheduled flows: compact records of each run, the alerts a run raises
//! compared with the one before, and the per-flow run log in
//! `.cactus/critters/<id>/runs/<flow>.json`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::critter::{AlertOn, AlertRule, validate_critter_id, validate_flow_id};
use crate::env::ensure_critter_workspace;
use crate::http::Check;
use crate::runner::{FlowRun, FlowStepResult};
use crate::storage::{ensure_parent_dir, read_json_lenient, write_json};
use crate::tools::{Finding, FindingKey, ToolStep};
use crate::{Result, layout};

/// How many runs a flow's log keeps.
pub const MAX_RUN_RECORDS: usize = 50;

/// One run, without bodies, so the log stays small.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RunRecord {
    /// Unix epoch millis.
    // A JS `number`, accepting precision loss: epoch millis stay far below 2^53.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub at: u64,
    pub env: Option<String>,
    pub ok: bool,
    pub elapsed_ms: u32,
    pub steps: Vec<StepRecord>,
    /// Why the run couldn't start, e.g. the flow file no longer parses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StepRecord {
    pub request: String,
    pub ok: bool,
    pub skipped: bool,
    pub status: Option<u16>,
    pub failed_checks: Vec<Check>,
    pub error: Option<String>,
    pub watched: BTreeMap<String, Option<String>>,
    /// What a security tool found; `None` when the step didn't scan, so
    /// an empty list means "found nothing".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub findings: Option<Vec<FindingKey>>,
    /// A tool's metrics, by name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metrics: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum AlertEvent {
    /// The flow started failing. `step` is `None` when it couldn't start.
    Failing {
        step: Option<u32>,
        request: Option<String>,
        reason: String,
    },
    Recovered,
    /// A watched value differs from the previous run's.
    Changed {
        step: u32,
        expression: String,
        before: Option<String>,
        after: Option<String>,
    },
    /// A tool found things the previous run's same step didn't.
    NewFindings {
        step: u32,
        request: String,
        findings: Vec<FindingKey>,
    },
}

impl RunRecord {
    /// A run that failed before any step, e.g. because the flow file no
    /// longer parses.
    pub fn errored(at: u64, env: Option<String>, error: String) -> Self {
        Self { at, env, ok: false, elapsed_ms: 0, steps: Vec::new(), error: Some(error) }
    }
}

impl StepRecord {
    fn reason(&self) -> String {
        if let Some(error) = &self.error {
            return error.clone();
        }
        match (self.failed_checks.first(), self.status) {
            (Some(check), _) => format!("{}: {}", check.name, check.detail),
            (None, Some(status)) => format!("status {status}"),
            (None, None) => "failed".into(),
        }
    }
}

pub fn record(run: &FlowRun) -> RunRecord {
    RunRecord {
        at: run.at,
        env: run.env.clone(),
        ok: run.ok,
        elapsed_ms: run.elapsed_ms,
        steps: run.steps.iter().map(step_record).collect(),
        error: None,
    }
}

fn step_record(step: &FlowStepResult) -> StepRecord {
    let response = step.response.as_ref();
    StepRecord {
        // A tool step is logged by its title, e.g. `POST https://…`.
        request: step.label().to_string(),
        ok: step.ok,
        skipped: step.skipped,
        status: response.map(|r| r.status).or_else(|| step.output.as_ref()?.status),
        failed_checks: response.map_or_default(|r| r.checks.iter().filter(|c| !c.ok).cloned().collect()),
        error: step.error.clone(),
        watched: step.watched.clone(),
        findings: step
            .output
            .as_ref()
            .filter(|_| is_scan(step))
            .map(|output| output.findings.iter().map(Finding::key).collect()),
        metrics: step
            .output
            .as_ref()
            .map_or_default(|output| output.metrics.iter().map(|metric| (metric.name.clone(), metric.value)).collect()),
    }
}

/// Whether a step is a tool that reports findings, so that finding
/// nothing is recorded as an empty list rather than left out.
fn is_scan(step: &FlowStepResult) -> bool {
    step.tool.as_deref().is_some_and(ToolStep::scans)
}

/// The alerts `current` raises after `previous`. Only changes of state
/// alert: the first run is the baseline, and a flow that keeps failing
/// alerts once. Watched values are compared only where both runs reached
/// the step and both have a value.
pub fn compare(previous: Option<&RunRecord>, current: &RunRecord, rule: &AlertRule) -> Vec<AlertEvent> {
    let Some(previous) = previous else { return Vec::new() };
    let mut events = Vec::new();
    if rule.on.contains(&AlertOn::Failure) {
        match (previous.ok, current.ok) {
            (true, false) => events.push(failing(current)),
            (false, true) => events.push(AlertEvent::Recovered),
            _ => {}
        }
    }
    if rule.on.contains(&AlertOn::Change) {
        let reached = |step: &&StepRecord| !step.skipped && step.error.is_none();
        for (index, (before, after)) in previous.steps.iter().zip(&current.steps).enumerate() {
            if !reached(&before) || !reached(&after) || before.request != after.request {
                continue;
            }
            for (expression, value) in &after.watched {
                let old = before.watched.get(expression).cloned().flatten();
                if let (Some(old), Some(new)) = (&old, value)
                    && old != new
                {
                    events.push(AlertEvent::Changed {
                        step: u32::try_from(index).unwrap_or(u32::MAX),
                        expression: expression.clone(),
                        before: Some(old.clone()),
                        after: Some(new.clone()),
                    });
                }
            }
        }
    }
    if rule.on.contains(&AlertOn::Findings) {
        events.extend(new_findings(previous, current));
    }
    events
}

/// Findings each step has that the previous run's same step didn't, where
/// both runs got as far as scanning.
fn new_findings(previous: &RunRecord, current: &RunRecord) -> Vec<AlertEvent> {
    let steps = previous.steps.iter().zip(&current.steps).enumerate();
    steps
        .filter_map(|(index, (before, after))| {
            if before.request != after.request {
                return None;
            }
            let (Some(old), Some(new)) = (&before.findings, &after.findings) else { return None };
            let fresh: Vec<FindingKey> =
                new.iter().filter(|key| !old.iter().any(|seen| seen.same_as(key))).cloned().collect();
            (!fresh.is_empty()).then(|| AlertEvent::NewFindings {
                step: u32::try_from(index).unwrap_or(u32::MAX),
                request: after.request.clone(),
                findings: fresh,
            })
        })
        .collect()
}

fn failing(run: &RunRecord) -> AlertEvent {
    let failed = run.steps.iter().enumerate().find(|(_, step)| !step.ok && !step.skipped);
    match failed {
        Some((index, step)) => AlertEvent::Failing {
            step: u32::try_from(index).ok(),
            request: Some(step.request.clone()),
            reason: step.reason(),
        },
        None => AlertEvent::Failing {
            step: None,
            request: None,
            reason: run.error.clone().unwrap_or_else(|| "failed".into()),
        },
    }
}

/// Guards the read-modify-write of run logs: a timer run and "Run now" can
/// finish together.
static LOG_WRITE: Mutex<()> = Mutex::new(());

/// Add `record` to the flow's log, keeping the newest [`MAX_RUN_RECORDS`].
pub fn append(project: &Path, critter: &str, flow: &str, record: RunRecord) -> Result<()> {
    validate_critter_id(critter)?;
    validate_flow_id(flow)?;
    let _guard = LOG_WRITE.lock().unwrap_or_else(PoisonError::into_inner);
    let file = layout::run_log_file(project, critter, flow);
    let mut log: Vec<RunRecord> = read_json_lenient(&file);
    log.push(record);
    let excess = log.len().saturating_sub(MAX_RUN_RECORDS);
    log.drain(..excess);
    ensure_critter_workspace(project, critter)?;
    ensure_parent_dir(&file)?;
    write_json(&file, &log)
}

/// The flow's runs, oldest first. A missing or corrupt log is empty.
pub fn load_log(project: &Path, critter: &str, flow: &str) -> Vec<RunRecord> {
    if validate_critter_id(critter).is_err() || validate_flow_id(flow).is_err() {
        return Vec::new();
    }
    read_json_lenient(&layout::run_log_file(project, critter, flow))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Severity;

    fn step(ok: bool, watched: &[(&str, Option<&str>)]) -> StepRecord {
        StepRecord {
            request: "status.yml".into(),
            ok,
            skipped: false,
            status: Some(if ok { 200 } else { 500 }),
            failed_checks: Vec::new(),
            error: None,
            watched: watched.iter().map(|(k, v)| ((*k).to_string(), v.map(str::to_string))).collect(),
            findings: None,
            metrics: BTreeMap::new(),
        }
    }

    fn scan(rules: Option<&[&str]>) -> RunRecord {
        let mut scanned = step(true, &[]);
        scanned.findings = rules.map(|rules| {
            let key = |rule: &&str| FindingKey { severity: Severity::High, rule: (*rule).into(), location: "/".into() };
            rules.iter().map(key).collect()
        });
        run(vec![scanned])
    }

    #[test]
    fn alerts_on_findings_the_previous_scan_did_not_have() {
        let findings = rule(&[AlertOn::Findings]);
        assert_eq!(compare(None, &scan(Some(&["hsts"])), &findings), QUIET, "the first run is the baseline");
        assert_eq!(compare(Some(&scan(Some(&["hsts"]))), &scan(Some(&["hsts"])), &findings), QUIET);
        assert_eq!(compare(Some(&scan(Some(&["hsts"]))), &scan(Some(&[])), &findings), QUIET);
        let events = compare(Some(&scan(Some(&["hsts"]))), &scan(Some(&["hsts", "csp"])), &findings);
        assert!(
            matches!(&events[..], [AlertEvent::NewFindings { step: 0, findings, .. }] if findings.len() == 1 && findings[0].rule == "csp")
        );
        assert_eq!(compare(Some(&scan(None)), &scan(Some(&["csp"])), &findings), QUIET, "the step didn't scan before");
        assert_eq!(compare(Some(&scan(Some(&[]))), &scan(Some(&["csp"])), &AlertRule::default()), QUIET);
    }

    fn run(steps: Vec<StepRecord>) -> RunRecord {
        let ok = steps.iter().all(|s| s.ok);
        RunRecord { at: 1, env: None, ok, elapsed_ms: 1, steps, error: None }
    }

    const QUIET: Vec<AlertEvent> = Vec::new();
    const NO_RUNS: Vec<RunRecord> = Vec::new();

    fn rule(on: &[AlertOn]) -> AlertRule {
        AlertRule { on: on.to_vec() }
    }

    #[test]
    fn alerts_on_changes_of_state_only() {
        let both = rule(&[AlertOn::Failure, AlertOn::Change]);
        let ok = run(vec![step(true, &[])]);
        let failing = run(vec![step(false, &[])]);
        assert_eq!(compare(None, &failing, &both), QUIET, "the first run is the baseline");
        assert_eq!(compare(Some(&ok), &ok, &both), QUIET);
        assert_eq!(
            compare(Some(&ok), &failing, &both),
            [AlertEvent::Failing { step: Some(0), request: Some("status.yml".into()), reason: "status 500".into() }]
        );
        assert_eq!(compare(Some(&failing), &failing, &both), QUIET);
        assert_eq!(compare(Some(&failing), &ok, &both), [AlertEvent::Recovered]);
        assert_eq!(compare(Some(&ok), &failing, &rule(&[AlertOn::Change])), QUIET);
    }

    #[test]
    fn failing_names_the_first_failed_check() {
        let mut bad = step(false, &[]);
        bad.failed_checks = vec![Check { name: "res.body.db".into(), ok: false, detail: "expected eq ok".into() }];
        let ok = run(vec![step(true, &[])]);
        let events = compare(Some(&ok), &run(vec![step(true, &[]), bad]), &AlertRule::default());
        assert!(
            matches!(&events[..], [AlertEvent::Failing { step: Some(1), reason, .. }] if reason == "res.body.db: expected eq ok")
        );
        let broken = RunRecord::errored(2, None, "yaml: bad".into());
        assert_eq!(
            compare(Some(&ok), &broken, &AlertRule::default()),
            [AlertEvent::Failing { step: None, request: None, reason: "yaml: bad".into() }]
        );
    }

    #[test]
    fn watched_changes_skip_missing_values_and_unreached_steps() {
        let change = rule(&[AlertOn::Change]);
        let v = |value: Option<&str>| run(vec![step(true, &[("res.body.version", value)])]);
        assert_eq!(
            compare(Some(&v(Some("1.4.2"))), &v(Some("1.5.0")), &change),
            [AlertEvent::Changed {
                step: 0,
                expression: "res.body.version".into(),
                before: Some("1.4.2".into()),
                after: Some("1.5.0".into())
            }]
        );
        assert_eq!(compare(Some(&v(Some("1"))), &v(None), &change), QUIET);
        assert_eq!(compare(Some(&v(None)), &v(Some("1")), &change), QUIET);
        let mut skipped = v(Some("2"));
        skipped.steps[0].skipped = true;
        assert_eq!(compare(Some(&v(Some("1"))), &skipped, &change), QUIET);
        assert_eq!(compare(Some(&v(Some("1"))), &v(Some("2")), &AlertRule::default()), QUIET);
    }

    #[test]
    fn log_keeps_the_newest_records() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        assert_eq!(load_log(p, "bob", "health"), NO_RUNS);
        for at in 0..(MAX_RUN_RECORDS as u64 + 5) {
            append(p, "bob", "health", RunRecord { at, ..run(vec![]) }).unwrap();
        }
        let log = load_log(p, "bob", "health");
        assert_eq!((log.len(), log[0].at, log.last().unwrap().at), (MAX_RUN_RECORDS, 5, 54));
        std::fs::write(layout::run_log_file(p, "bob", "health"), "not json").unwrap();
        assert_eq!(load_log(p, "bob", "health"), NO_RUNS);
        assert!(append(p, "../x", "health", run(vec![])).is_err());
    }
}
