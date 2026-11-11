//! Running a critter's flow: its steps in order, each an overlay on an
//! existing request.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::{SendOptions, send_and_record, send_as};
use crate::clock::now_unix_millis;
use crate::critter::{self, FlowStep};
use crate::env::{self, Scope};
use crate::http::{Response, is_status_expression, scenario_passed, select_values};
use crate::scenario::{self, Action, Assertion, Scenario, SetVariable};
use crate::{Error, ProjectPath, Result, layout};

/// The outcome of one step. Every step of a run gets one, so the UI and the
/// run log can line them up with the flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FlowStepResult {
    pub index: u32,
    /// The step's `request`, as written.
    pub request: String,
    pub title: String,
    pub ok: bool,
    /// Not run because an earlier step failed.
    pub skipped: bool,
    pub response: Option<Response>,
    pub error: Option<String>,
    /// The values of the step's `watch` expressions.
    pub watched: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FlowRun {
    /// Unix epoch millis when the run started.
    // A JS `number`, accepting precision loss: epoch millis stay far below 2^53.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub at: u64,
    pub env: Option<String>,
    pub ok: bool,
    pub elapsed_ms: u32,
    pub steps: Vec<FlowStepResult>,
}

impl FlowRun {
    /// Why the run failed, naming the first failed step: its first failed
    /// check, its error, or its status.
    pub fn failure(&self) -> Option<String> {
        let step = self.steps.iter().find(|step| !step.ok && !step.skipped)?;
        Some(format!("step {} ({}): {}", step.index + 1, step.request, step.reason()))
    }
}

impl FlowStepResult {
    fn skipped(index: usize, step: &FlowStep) -> Self {
        let mut result = Self::new(index, step);
        result.skipped = true;
        result
    }

    fn errored(index: usize, step: &FlowStep, error: &Error) -> Self {
        let mut result = Self::new(index, step);
        result.error = Some(error.to_string());
        result
    }

    fn new(index: usize, step: &FlowStep) -> Self {
        let request = step.request.trim().to_string();
        Self {
            index: u32::try_from(index).unwrap_or(u32::MAX),
            title: request.clone(),
            request,
            ok: false,
            skipped: false,
            response: None,
            error: None,
            watched: BTreeMap::new(),
        }
    }

    /// Why this step failed.
    pub fn reason(&self) -> String {
        if let Some(error) = &self.error {
            return error.clone();
        }
        let Some(response) = &self.response else { return "not run".into() };
        match response.checks.iter().find(|check| !check.ok) {
            Some(check) => format!("{}: {}", check.name, check.detail),
            None => format!("status {}", response.status),
        }
    }
}

/// The request as the step runs it: `expect` replaces its status
/// assertions, `assert` adds checks and `capture` adds set-variable
/// actions. The file on disk is never touched.
pub fn apply_step(mut scenario: Scenario, step: &FlowStep) -> Scenario {
    let runtime = &mut scenario.runtime;
    if let Some(status) = step.expect {
        runtime.assertions.retain(|a| !is_status_expression(&a.expression));
        runtime.assertions.push(Assertion {
            expression: "res.status".into(),
            operator: "eq".into(),
            value: Some(status.to_string().into()),
            disabled: false,
        });
    }
    runtime.assertions.extend(step.assert.iter().cloned());
    let captures = step.capture.iter().map(|(name, expression)| SetVariable::new(name.clone(), expression.0.clone()));
    runtime.actions.extend(captures.map(Action::SetVariable));
    scenario
}

/// How a flow's steps are sent.
pub(super) trait StepSender {
    fn send(
        &self,
        client: &reqwest::Client,
        project: &Path,
        path: &ProjectPath,
        scenario: &Scenario,
        scope: &Scope,
        options: SendOptions,
    ) -> impl Future<Output = Result<Response>> + Send;
}

/// Without logging in again: for the login flow itself.
pub(super) struct Plain;

/// With [`send_as`]: a 401 logs the critter in again.
struct Relogin;

impl StepSender for Plain {
    async fn send(
        &self,
        client: &reqwest::Client,
        project: &Path,
        path: &ProjectPath,
        scenario: &Scenario,
        scope: &Scope,
        options: SendOptions,
    ) -> Result<Response> {
        send_and_record(client, project, path, scenario, scope, options).await
    }
}

impl StepSender for Relogin {
    async fn send(
        &self,
        client: &reqwest::Client,
        project: &Path,
        path: &ProjectPath,
        scenario: &Scenario,
        scope: &Scope,
        options: SendOptions,
    ) -> Result<Response> {
        send_as(client, project, path, scenario, scope, options).await
    }
}

/// Run `critter`'s flow `flow` in env `env` (the active one when `None`),
/// calling `on_step` as each step finishes. A failed step stops the flow;
/// the steps after it are reported as skipped.
pub async fn run_flow(
    client: &reqwest::Client,
    project: &Path,
    critter: &str,
    flow: &str,
    env: Option<String>,
    options: SendOptions,
    on_step: impl FnMut(&FlowStepResult) + Send,
) -> Result<FlowRun> {
    let scope = Scope { env: env.or_else(|| env::active(project)), critter: Some(critter.into()), ..Scope::default() };
    run_steps(&Relogin, client, project, critter, flow, &scope, options, on_step).await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_steps(
    sender: &impl StepSender,
    client: &reqwest::Client,
    project: &Path,
    critter: &str,
    flow: &str,
    scope: &Scope,
    options: SendOptions,
    mut on_step: impl FnMut(&FlowStepResult) + Send,
) -> Result<FlowRun> {
    if !critter::exists(project, critter) {
        critter::validate_critter_id(critter)?;
        return Err(Error::CritterNotFound(critter.into()));
    }
    let definition = critter::load_flow(project, critter, flow)?;
    let (at, started) = (now_unix_millis(), Instant::now());
    let mut steps = Vec::with_capacity(definition.steps.len());
    let mut failed = false;
    for (index, step) in definition.steps.iter().enumerate() {
        let result = if failed {
            FlowStepResult::skipped(index, step)
        } else {
            run_step(sender, client, project, index, step, scope, options).await
        };
        failed = !result.ok;
        on_step(&result);
        steps.push(result);
    }
    let elapsed_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    Ok(FlowRun { at, env: scope.env.clone(), ok: !failed, elapsed_ms, steps })
}

async fn run_step(
    sender: &impl StepSender,
    client: &reqwest::Client,
    project: &Path,
    index: usize,
    step: &FlowStep,
    scope: &Scope,
    options: SendOptions,
) -> FlowStepResult {
    let loaded = ProjectPath::parse(step.request.trim()).and_then(|path| {
        if !layout::is_request_file(path.as_str()) {
            return Err(Error::InvalidPath(path.to_string()));
        }
        let scenario = scenario::load(project, &path)?;
        Ok((path, scenario))
    });
    let (path, scenario) = match loaded {
        Ok(loaded) => loaded,
        Err(error) => return FlowStepResult::errored(index, step, &error),
    };
    let mut result = FlowStepResult::new(index, step);
    if !scenario.info.name.trim().is_empty() {
        result.title = scenario.info.name.trim().to_string();
    }
    let scenario = apply_step(scenario, step);
    let overrides = step.with.iter().map(|(name, value)| (name.clone(), value.0.clone())).collect();
    let scope = Scope { overrides, ..scope.clone() };
    match sender.send(client, project, &path, &scenario, &scope, options).await {
        Ok(response) => {
            let watch: Vec<String> = step.watch.iter().map(|w| w.trim().to_string()).collect();
            result.ok = scenario_passed(&scenario, &response);
            result.watched = select_values(&response, &watch);
            result.response = Some(response);
        }
        Err(error) => result.error = Some(error.to_string()),
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::LenientString;

    fn assertion(expression: &str, operator: &str, value: &str) -> Assertion {
        Assertion {
            expression: expression.into(),
            operator: operator.into(),
            value: Some(value.into()),
            disabled: false,
        }
    }

    #[test]
    fn overlay_replaces_status_checks_and_adds_the_rest() {
        let mut scenario = Scenario::default();
        scenario.runtime.assertions =
            vec![assertion("res.status", "eq", "200"), assertion("res.body.ok", "eq", "true")];
        let step = FlowStep {
            expect: Some(403),
            assert: vec![assertion("res.body.db", "eq", "ok")],
            capture: BTreeMap::from([("token".into(), LenientString::from("res.body.token"))]),
            ..FlowStep::default()
        };
        let overlaid = apply_step(scenario, &step);
        let checks: Vec<_> = overlaid
            .runtime
            .assertions
            .iter()
            .map(|a| (a.expression.as_str(), a.value.as_deref().map(String::as_str)))
            .collect();
        assert_eq!(checks, [("res.body.ok", Some("true")), ("res.status", Some("403")), ("res.body.db", Some("ok"))]);
        assert_eq!(overlaid.runtime.actions, [Action::SetVariable(SetVariable::new("token", "res.body.token"))]);

        let untouched = apply_step(Scenario::default(), &FlowStep::default());
        assert_eq!(untouched, Scenario::default());
    }
}
