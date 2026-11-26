//! Sending requests with their side effects: captured variables, history
//! and, for critters, logging in again. Also folder runs, flows and the
//! matrix of one request sent as every critter.

#[cfg(test)]
mod auth_tests;
mod flow;
mod matrix;
#[cfg(test)]
mod test_server;
#[cfg(test)]
mod tests;

use std::path::Path;

use serde::{Deserialize, Serialize};

pub use self::flow::{FlowEvent, FlowRun, FlowStepResult, FlowStepStart, apply_step, run_flow, run_flow_definition};
pub use self::matrix::{MatrixCell, run_matrix};
use crate::best_effort;
use crate::clock::now_unix_millis;
use crate::env::{RequestScope, Scope};
use crate::history::{self, HistoryEntry};
use crate::http::{self, Response, scenario_passed};
use crate::oauth2::{self, StoredToken};
use crate::scenario::{self, Scenario, ScenarioEntry};
use crate::tools::Headless;
use crate::{Error, ProjectPath, Result, cookies, critter, env};

/// The status that makes a critter log in again.
const UNAUTHORIZED: u16 = 401;

/// Whether a send may change the critter's credentials: keep the cookies
/// responses set, and replace an OAuth2 token the server refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Credentials {
    Update,
    ReadOnly,
}

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
    let mut response = send_with_credentials(client, project, scenario, &resolved, scope, Credentials::Update).await?;
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

/// Send one request as [`send_and_record`] does, but keep nothing: no
/// history, and neither captured values nor cookies are saved. Never logs
/// in again or replaces an OAuth2 token on a 401, which a probe may be
/// looking for. For tools that send many variants of a request, such as
/// fuzzers and load tests.
pub(crate) async fn send_probe(
    client: &reqwest::Client,
    project: &Path,
    path: &ProjectPath,
    scenario: &Scenario,
    scope: &Scope,
) -> Result<Response> {
    let resolved = env::resolve_for_request(project, scope, path.dir(), &scenario.runtime.variables)?;
    let mut response =
        send_with_credentials(client, project, scenario, &resolved, scope, Credentials::ReadOnly).await?;
    response.critter.clone_from(&scope.critter);
    Ok(response)
}

/// Send with the request's OAuth2 token, if it uses one, and the critter's
/// cookie jar, if there is a critter. With [`Credentials::Update`], a cached
/// token the server refuses with a 401 is refreshed or replaced, and the
/// request sent once more.
async fn send_with_credentials(
    client: &reqwest::Client,
    project: &Path,
    scenario: &Scenario,
    resolved: &RequestScope,
    scope: &Scope,
    credentials: Credentials,
) -> Result<Response> {
    let critter = scope.critter.as_deref();
    let send = |token| send_once(client, project, scenario, resolved, critter, token, credentials);
    let Some(oauth2) = http::effective_oauth2(scenario, &resolved.inherited) else {
        return send(None).await;
    };
    let token = oauth2::token_for_send(client, project, critter, oauth2, &resolved.vars).await?;
    let response = send(Some(&token.token)).await?;
    if response.status != UNAUTHORIZED || token.fresh || credentials == Credentials::ReadOnly {
        return Ok(response);
    }
    oauth2::expire(project, critter, oauth2, &resolved.vars)?;
    match oauth2::token_for_send(client, project, critter, oauth2, &resolved.vars).await {
        Ok(token) => {
            let mut again = send(Some(&token.token)).await?;
            again.warnings.push("Got 401 with the cached OAuth2 token; sent again with a new one".into());
            Ok(again)
        }
        Err(error) => {
            let mut response = response;
            response.warnings.push(format!("Got 401 with the cached OAuth2 token; {error}"));
            Ok(response)
        }
    }
}

async fn send_once(
    client: &reqwest::Client,
    project: &Path,
    scenario: &Scenario,
    resolved: &RequestScope,
    critter: Option<&str>,
    token: Option<&StoredToken>,
    credentials: Credentials,
) -> Result<Response> {
    let send =
        http::send(client, project, scenario, &resolved.inherited, &resolved.vars, token.map(StoredToken::bearer));
    match critter {
        Some(critter) => cookies::with_jar(project, critter, credentials == Credentials::Update, send).await?,
        None => send.await,
    }
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
    // Logging in happens mid-request, with no one to ask: a prompt in the
    // login flow uses its defaults.
    let context =
        flow::StepContext { client, project, scope: &base, options, relogin: false, host: &Headless, depth: 0 };
    let failure = match critter::load_flow(project, id, login) {
        Ok(definition) => match flow::run_steps(&context, &definition, |_| {}).await {
            Ok(run) => run.failure(),
            Err(error) => Some(error.to_string()),
        },
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
    best_effort::log("record history", history::push(project, path, entry));
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
