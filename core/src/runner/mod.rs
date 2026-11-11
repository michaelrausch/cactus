//! Sending requests with their side effects: captured variables, history
//! and, for critters, logging in again. Also folder runs, flows and the
//! matrix of one request sent as every critter.

mod flow;
mod matrix;
#[cfg(test)]
mod test_server;
#[cfg(test)]
mod tests;

use std::path::Path;

use serde::{Deserialize, Serialize};

pub use self::flow::{FlowRun, FlowStepResult, apply_step, run_flow};
pub use self::matrix::{MatrixCell, run_matrix};
use crate::clock::now_unix_millis;
use crate::env::Scope;
use crate::history::{self, HistoryEntry};
use crate::http::{self, Response, scenario_passed};
use crate::scenario::{self, Scenario, ScenarioEntry};
use crate::{Error, ProjectPath, Result, critter, env};

/// The status that makes a critter log in again.
const UNAUTHORIZED: u16 = 401;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendOptions {
    pub record_history: bool,
}

impl SendOptions {
    /// Sends the user asked for, which go into history.
    pub const INTERACTIVE: Self = Self { record_history: true };
    /// Scheduled runs and the matrix, which don't.
    pub const BACKGROUND: Self = Self { record_history: false };
}

/// Send one request with what its collection and folders set and the
/// variables of `scope`, persist the variables its actions set (to the
/// critter's context when there is one) and maybe record it in history. The
/// response is stamped with the critter.
pub async fn send_and_record(
    client: &reqwest::Client,
    project: &Path,
    path: &ProjectPath,
    scenario: &Scenario,
    scope: &Scope,
    options: SendOptions,
) -> Result<Response> {
    let resolved = env::resolve_for_request(project, scope, path.dir(), &scenario.runtime.variables)?;
    let mut response = http::send(client, project, scenario, &resolved.inherited, &resolved.vars).await?;
    response.critter.clone_from(&scope.critter);
    if !response.captured.is_empty() {
        match &scope.critter {
            Some(critter) => env::merge_context(project, critter, &response.captured, path)?,
            None => env::merge_captured(project, &response.captured)?,
        }
    }
    if options.record_history {
        record_history(project, path, &response);
    }
    Ok(response)
}

/// [`send_and_record`], and when a critter gets a 401 and has a `login`
/// flow: run that flow, and if it passes send the request once more with
/// the fresh context.
///
/// The login flow's steps go through [`send_and_record`] directly, so a 401
/// inside the login flow never logs in again.
pub async fn send_as(
    client: &reqwest::Client,
    project: &Path,
    path: &ProjectPath,
    scenario: &Scenario,
    scope: &Scope,
    options: SendOptions,
) -> Result<Response> {
    let mut response = send_and_record(client, project, path, scenario, scope, options).await?;
    if response.status != UNAUTHORIZED {
        return Ok(response);
    }
    let Some(id) = &scope.critter else { return Ok(response) };
    let Some(login) = critter::load(project, id)?.login.filter(|login| !login.trim().is_empty()) else {
        return Ok(response);
    };
    let login = login.trim();
    let base = Scope { overrides: env::Env::new(), ..scope.clone() };
    let failure = match flow::run_steps(
        &flow::Plain,
        client,
        project,
        id,
        login,
        &base,
        options,
        |_: &FlowStepResult| {},
    )
    .await
    {
        Ok(run) => run.failure(),
        Err(error) => Some(error.to_string()),
    };
    if let Some(reason) = failure {
        response.warnings.push(format!("Got 401; logging in again with flow `{login}` failed: {reason}"));
        return Ok(response);
    }
    let mut again = send_and_record(client, project, path, scenario, scope, options).await?;
    again.relogged = true;
    again.warnings.push(format!("Logged in again after 401 using flow `{login}`; the request was sent twice"));
    Ok(again)
}

/// History is a convenience; failing to write it shouldn't fail the request.
fn record_history(project: &Path, path: &ProjectPath, response: &Response) {
    let entry = HistoryEntry { at: now_unix_millis(), response: response.clone() };
    let _ = history::push(project, path, entry);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RunResult {
    pub rel_path: ProjectPath,
    pub title: String,
    pub ok: bool,
    pub response: Option<Response>,
    pub error: Option<String>,
}

impl RunResult {
    fn completed(entry: ScenarioEntry, scenario: &Scenario, response: Response) -> Self {
        let ok = scenario_passed(scenario, &response);
        Self { rel_path: entry.rel_path, title: entry.title, ok, response: Some(response), error: None }
    }

    fn errored(entry: ScenarioEntry, error: &Error) -> Self {
        Self { rel_path: entry.rel_path, title: entry.title, ok: false, response: None, error: Some(error.to_string()) }
    }
}

/// Run every request under `dir` (all when empty) in listing order as
/// `scope`, calling `on_result` as each finishes. Variables set by earlier
/// requests are visible to later ones.
pub async fn run(
    client: &reqwest::Client,
    project: &Path,
    dir: &str,
    scope: &Scope,
    mut on_result: impl FnMut(&RunResult),
) -> Result<Vec<RunResult>> {
    let entries = scenario::list(project)?.into_iter().filter(|entry| entry.rel_path.is_within(dir));
    let mut results = Vec::new();
    for entry in entries {
        let result = run_one(client, project, entry, scope).await;
        on_result(&result);
        results.push(result);
    }
    Ok(results)
}

async fn run_one(client: &reqwest::Client, project: &Path, entry: ScenarioEntry, scope: &Scope) -> RunResult {
    let scenario = match scenario::load(project, &entry.rel_path) {
        Ok(scenario) => scenario,
        Err(error) => return RunResult::errored(entry, &error),
    };
    match send_as(client, project, &entry.rel_path, &scenario, scope, SendOptions::INTERACTIVE).await {
        Ok(response) => RunResult::completed(entry, &scenario, response),
        Err(error) => RunResult::errored(entry, &error),
    }
}
