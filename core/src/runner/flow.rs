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
use crate::http::{Response, is_status_expression, preview_url, scenario_passed, select_values};
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
    /// The method and URL it was sent to, env applied; empty when the
    /// request couldn't be read.
    pub method: String,
    pub url: String,
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

/// A step about to send: where it's going.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FlowStepStart {
    pub index: u32,
    pub request: String,
    pub title: String,
    pub method: String,
    pub url: String,
}

/// What a run reports as it goes. A step that is skipped, or whose request
/// can't be read, finishes without starting.
#[derive(Debug, Clone, Copy)]
pub enum FlowEvent<'a> {
    Started(&'a FlowStepStart),
    Finished(&'a FlowStepResult),
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
            method: String::new(),
            url: String::new(),
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
/// calling `on_event` as each step starts and finishes. A failed step stops
/// the flow; the steps after it are reported as skipped.
pub async fn run_flow(
    client: &reqwest::Client,
    project: &Path,
    critter: &str,
    flow: &str,
    env: Option<String>,
    options: SendOptions,
    on_event: impl FnMut(FlowEvent<'_>) + Send,
) -> Result<FlowRun> {
    super::ensure_unlocked_for(project, Some(critter))?;
    let scope = Scope { env: env.or_else(|| env::active(project)), critter: Some(critter.into()), ..Scope::default() };
    run_steps(&Relogin, client, project, critter, flow, &scope, options, on_event).await
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
    mut on_event: impl FnMut(FlowEvent<'_>) + Send,
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
            run_step(sender, client, project, index, step, scope, options, &mut on_event).await
        };
        failed = !result.ok;
        on_event(FlowEvent::Finished(&result));
        steps.push(result);
    }
    let elapsed_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    Ok(FlowRun { at, env: scope.env.clone(), ok: !failed, elapsed_ms, steps })
}

#[allow(clippy::too_many_arguments)]
async fn run_step(
    sender: &impl StepSender,
    client: &reqwest::Client,
    project: &Path,
    index: usize,
    step: &FlowStep,
    scope: &Scope,
    options: SendOptions,
    on_event: &mut (impl FnMut(FlowEvent<'_>) + Send),
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
    // `with` values are filled in from what the step sees without them, so
    // `COURSE: '{{code}}'` passes on the value an earlier step captured.
    // Substitution is single pass, so left raw they'd be sent as `{{code}}`.
    let base = env::resolve_for_request(project, scope, path.dir(), &scenario.runtime.variables).ok();
    let overrides: env::Env = step
        .with
        .iter()
        .map(|(name, value)| {
            let value = match &base {
                Some(base) => env::Substituter::new(&base.vars).apply(value),
                None => value.0.clone(),
            };
            (name.clone(), value)
        })
        .collect();
    result.method = scenario.http.method.trim().to_uppercase();
    result.url = match base {
        Some(base) => {
            let mut vars = base.vars;
            vars.extend(overrides.clone());
            preview_url(&scenario, &vars)
        }
        None => scenario.http.url.trim().to_string(),
    };
    let scope = Scope { overrides, ..scope.clone() };
    let start = FlowStepStart {
        index: result.index,
        request: result.request.clone(),
        title: result.title.clone(),
        method: result.method.clone(),
        url: result.url.clone(),
    };
    on_event(FlowEvent::Started(&start));
    match sender.send(client, project, &path, &scenario, &scope, options).await {
        Ok(response) => {
            // What was actually sent, e.g. with an API key added to the query.
            result.url.clone_from(&response.url);
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
