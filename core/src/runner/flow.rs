//! Running a critter's flow: its steps in order, each an overlay on an
//! existing request or a tool.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::{SendOptions, send_and_record, send_as, send_probe};
use crate::clock::now_unix_millis;
use crate::critter::{self, Flow, FlowStep, Step};
use crate::env::{self, Env, Scope};
use crate::http::{Response, is_status_expression, preview_url, scenario_passed, select_values};
use crate::num::saturating_u32;
use crate::scenario::{self, Action, Assertion, Scenario, SetVariable};
use crate::tools::{BoxFuture, FlowRunner, ToolCx, ToolHost, ToolOutput, ToolStep};
use crate::{Error, ProjectPath, Result, layout};

/// The outcome of one step. Every step of a run gets one, so the UI and the
/// run log can line them up with the flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FlowStepResult {
    pub index: u32,
    /// The step's `request`, as written; empty for a tool.
    pub request: String,
    /// The tool's id, for a tool step.
    #[serde(default)]
    pub tool: Option<String>,
    /// What a tool step did.
    #[serde(default)]
    pub output: Option<ToolOutput>,
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
    pub tool: Option<String>,
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
        Some(format!("step {} ({}): {}", step.index + 1, step.label(), step.reason()))
    }
}

impl FlowStepResult {
    fn skipped(index: usize, step: &Step) -> Self {
        let mut result = Self::new(index, step);
        result.skipped = true;
        result
    }

    fn errored(index: usize, step: &Step, error: &Error) -> Self {
        let mut result = Self::new(index, step);
        result.error = Some(error.to_string());
        result
    }

    fn new(index: usize, step: &Step) -> Self {
        let (request, title, tool) = match step {
            Step::Request(step) => {
                let request = step.request.trim().to_string();
                (request.clone(), request, None)
            }
            Step::Tool(tool) => (String::new(), tool.title(), Some(tool.id().to_string())),
        };
        Self {
            index: saturating_u32(index),
            title,
            request,
            tool,
            output: None,
            method: String::new(),
            url: String::new(),
            ok: false,
            skipped: false,
            response: None,
            error: None,
            watched: BTreeMap::new(),
        }
    }

    /// The result of the step `start` describes, before it has been sent.
    fn started(start: &FlowStepStart) -> Self {
        Self {
            index: start.index,
            request: start.request.clone(),
            tool: start.tool.clone(),
            output: None,
            title: start.title.clone(),
            method: start.method.clone(),
            url: start.url.clone(),
            ok: false,
            skipped: false,
            response: None,
            error: None,
            watched: BTreeMap::new(),
        }
    }

    /// The request it sent, or the tool's title.
    pub fn label(&self) -> &str {
        if self.request.is_empty() { &self.title } else { &self.request }
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

/// What the steps of one run share.
pub(super) struct StepContext<'a> {
    pub client: &'a reqwest::Client,
    pub project: &'a Path,
    pub scope: &'a Scope,
    pub options: SendOptions,
    /// A 401 logs the critter in again ([`send_as`]). Off for the login flow
    /// itself, so it can't loop.
    pub relogin: bool,
    /// Who tool steps ask, notify and open URLs for.
    pub host: &'a dyn ToolHost,
    /// How many `run` steps deep this is, so flows running each other stop.
    pub depth: u8,
}

/// How deep `- tool: run` may nest: plenty for setup flows, few enough that
/// two flows running each other fail fast.
const MAX_DEPTH: u8 = 8;

impl StepContext<'_> {
    async fn send(&self, path: &ProjectPath, scenario: &Scenario, scope: &Scope) -> Result<Response> {
        if self.relogin {
            // Boxed: logging in runs a flow, which sends steps, which is this.
            Box::pin(send_as(self.client, self.project, path, scenario, scope, self.options)).await
        } else {
            send_and_record(self.client, self.project, path, scenario, scope, self.options).await
        }
    }
}

/// Run `critter`'s flow `flow` in env `env` (the active one when `None`),
/// calling `on_event` as each step starts and finishes. Tool steps talk to
/// `host`. A failed step stops the flow; the steps after it are reported as
/// skipped.
#[allow(clippy::too_many_arguments)]
pub async fn run_flow(
    client: &reqwest::Client,
    project: &Path,
    critter: &str,
    flow: &str,
    env: Option<String>,
    options: SendOptions,
    host: &dyn ToolHost,
    on_event: impl FnMut(FlowEvent<'_>) + Send,
) -> Result<FlowRun> {
    critter::require(project, critter)?;
    let definition = critter::load_flow(project, critter, flow)?;
    run_flow_definition(client, project, critter, &definition, env, options, host, on_event).await
}

/// [`run_flow`] for a definition the caller has already loaded, so what it
/// decides from the definition (alert rule, env) is what runs.
#[allow(clippy::too_many_arguments)]
pub async fn run_flow_definition(
    client: &reqwest::Client,
    project: &Path,
    critter: &str,
    definition: &Flow,
    env: Option<String>,
    options: SendOptions,
    host: &dyn ToolHost,
    on_event: impl FnMut(FlowEvent<'_>) + Send,
) -> Result<FlowRun> {
    let scope = Scope { env: env.or_else(|| env::active(project)), critter: Some(critter.into()), ..Scope::default() };
    let context = StepContext { client, project, scope: &scope, options, relogin: true, host, depth: 0 };
    run_steps(&context, definition, on_event).await
}

pub(super) async fn run_steps(
    context: &StepContext<'_>,
    definition: &Flow,
    mut on_event: impl FnMut(FlowEvent<'_>) + Send,
) -> Result<FlowRun> {
    let (at, started) = (now_unix_millis(), Instant::now());
    let mut steps = Vec::with_capacity(definition.steps.len());
    let mut failed = false;
    // What tools set: seen by every later step, below its own `with`, and
    // never written to disk.
    let mut run_vars = Env::new();
    for (index, step) in definition.steps.iter().enumerate() {
        let result = match step {
            _ if failed => FlowStepResult::skipped(index, step),
            Step::Request(request) => run_step(context, index, request, &run_vars, &mut on_event).await,
            Step::Tool(tool) => run_tool(context, index, tool, &mut run_vars, &mut on_event).await,
        };
        failed = !result.ok;
        on_event(FlowEvent::Finished(&result));
        steps.push(result);
    }
    let elapsed_ms = saturating_u32(started.elapsed().as_millis());
    Ok(FlowRun { at, env: context.scope.env.clone(), ok: !failed, elapsed_ms, steps })
}

/// A step's request, ready to send.
struct PreparedStep {
    path: ProjectPath,
    scenario: Scenario,
    /// The run's scope plus the step's `with` values.
    scope: Scope,
    start: FlowStepStart,
}

fn load_step_request(project: &Path, step: &FlowStep) -> Result<(ProjectPath, Scenario)> {
    let path = ProjectPath::parse(step.request.trim())?;
    if !layout::is_request_file(path.as_str()) {
        return Err(Error::InvalidPath(path.to_string()));
    }
    let scenario = scenario::load(project, &path)?;
    Ok((path, scenario))
}

/// The step's `with` values, filled in from what the step sees without them,
/// so `COURSE: '{{code}}'` passes on the value an earlier step captured.
/// Substitution is single pass, so left raw they'd be sent as `{{code}}`.
fn step_overrides(step: &FlowStep, visible: &env::Env) -> env::Env {
    let mut substituter = env::Substituter::new(visible);
    step.with.iter().map(|(name, value)| (name.clone(), substituter.apply(value))).collect()
}

/// The run's scope with what tools set so far as overrides.
fn scope_with(context: &StepContext<'_>, run_vars: &Env) -> Scope {
    let mut overrides = context.scope.overrides.clone();
    overrides.extend(run_vars.clone());
    Scope { overrides, ..context.scope.clone() }
}

fn prepare_step(context: &StepContext<'_>, index: usize, step: &FlowStep, run_vars: &Env) -> Result<PreparedStep> {
    let (path, scenario) = load_step_request(context.project, step)?;
    let scenario = apply_step(scenario, step);
    let scope = scope_with(context, run_vars);
    let base = env::resolve_for_request(context.project, &scope, path.dir(), &scenario.runtime.variables)?;
    let with = step_overrides(step, &base.vars);
    let mut vars = base.vars;
    vars.extend(with.clone());
    let request = step.request.trim().to_string();
    let name = scenario.info.name.trim();
    let start = FlowStepStart {
        index: saturating_u32(index),
        title: if name.is_empty() { request.clone() } else { name.to_string() },
        request,
        tool: None,
        method: scenario.http.method.trim().to_uppercase(),
        url: preview_url(&scenario, &vars),
    };
    let mut scope = scope;
    scope.overrides.extend(with);
    Ok(PreparedStep { path, scenario, scope, start })
}

/// Run a tool step: fill in its templates, run it, and keep what it set
/// for later steps.
async fn run_tool(
    context: &StepContext<'_>,
    index: usize,
    tool: &ToolStep,
    run_vars: &mut Env,
    on_event: &mut (impl FnMut(FlowEvent<'_>) + Send),
) -> FlowStepResult {
    let step = Step::Tool(tool.clone());
    let visible = match env::resolve_for_request(context.project, &scope_with(context, run_vars), "", &[]) {
        Ok(resolved) => resolved.vars,
        Err(error) => return FlowStepResult::errored(index, &step, &error),
    };
    let mut tool = tool.clone();
    let mut substituter = env::Substituter::new(&visible);
    for text in tool.templates() {
        *text = substituter.apply(text);
    }
    let start = FlowStepStart {
        index: saturating_u32(index),
        request: String::new(),
        tool: Some(tool.id().to_string()),
        title: tool.title(),
        method: String::new(),
        url: String::new(),
    };
    let mut result = FlowStepResult::started(&start);
    on_event(FlowEvent::Started(&start));
    let runner = RunAccess { context, run_vars: run_vars.clone() };
    let mut cx = ToolCx::new(context.client, context.host, &runner, &visible).at(saturating_u32(index));
    match tool.run(&mut cx).await {
        Ok(mut output) => {
            let (set, shown) = cx.finish();
            run_vars.extend(set);
            output.values = shown;
            if let Some(findings) = &mut output.findings {
                findings.sort_by_key(|finding| std::cmp::Reverse(finding.severity));
            }
            result.error = tool.verdict(&output);
            result.ok = result.error.is_none();
            result.output = Some(output);
        }
        Err(error) => result.error = Some(error.to_string()),
    }
    result
}

/// What a tool step can do with its run ([`FlowRunner`]): as the run is
/// when the step starts.
struct RunAccess<'c> {
    context: &'c StepContext<'c>,
    run_vars: Env,
}

impl FlowRunner for RunAccess<'_> {
    fn send<'a>(&'a self, step: &'a FlowStep) -> BoxFuture<'a, Result<(Response, bool)>> {
        Box::pin(async move {
            let prepared = prepare_step(self.context, 0, step, &self.run_vars)?;
            let response = self.context.send(&prepared.path, &prepared.scenario, &prepared.scope).await?;
            let passed = scenario_passed(&prepared.scenario, &response);
            Ok((response, passed))
        })
    }

    fn probe<'a>(
        &'a self,
        step: &'a FlowStep,
        edit: &'a (dyn Fn(&mut Scenario) + Sync),
    ) -> BoxFuture<'a, Result<Response>> {
        Box::pin(async move {
            let mut prepared = prepare_step(self.context, 0, step, &self.run_vars)?;
            edit(&mut prepared.scenario);
            send_probe(self.context.client, self.context.project, &prepared.path, &prepared.scenario, &prepared.scope)
                .await
        })
    }

    fn run_flow<'a>(&'a self, critter: Option<&'a str>, flow: &'a str) -> BoxFuture<'a, Result<FlowRun>> {
        Box::pin(async move {
            let context = self.context;
            if context.depth >= MAX_DEPTH {
                return Err(Error::Tool(format!("Flows run each other more than {MAX_DEPTH} deep")));
            }
            let critter = match critter {
                Some(critter) => critter.to_string(),
                None => context.scope.critter.clone().unwrap_or_default(),
            };
            critter::require(context.project, &critter)?;
            let definition = critter::load_flow(context.project, &critter, flow)?;
            let scope = Scope { env: context.scope.env.clone(), critter: Some(critter), ..Scope::default() };
            let child = StepContext { scope: &scope, relogin: true, depth: context.depth + 1, ..*context };
            run_steps(&child, &definition, |_| {}).await
        })
    }
}

async fn run_step(
    context: &StepContext<'_>,
    index: usize,
    step: &FlowStep,
    run_vars: &Env,
    on_event: &mut (impl FnMut(FlowEvent<'_>) + Send),
) -> FlowStepResult {
    let prepared = match prepare_step(context, index, step, run_vars) {
        Ok(prepared) => prepared,
        Err(error) => return FlowStepResult::errored(index, &Step::Request(step.clone()), &error),
    };
    let mut result = FlowStepResult::started(&prepared.start);
    on_event(FlowEvent::Started(&prepared.start));
    match context.send(&prepared.path, &prepared.scenario, &prepared.scope).await {
        Ok(response) => {
            // What was actually sent, e.g. with an API key added to the query.
            result.url.clone_from(&response.url);
            let watch: Vec<String> = step.watch.iter().map(|w| w.trim().to_string()).collect();
            result.ok = scenario_passed(&prepared.scenario, &response);
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
