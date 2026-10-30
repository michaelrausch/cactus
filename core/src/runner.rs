//! Sending requests with their side effects: captured variables and history.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::clock::now_unix_millis;
use crate::history::{self, HistoryEntry};
use crate::http::{self, Response, scenario_passed};
use crate::scenario::{self, Scenario, ScenarioEntry};
use crate::{Error, Result, ScenarioPath, collection, env};

/// Send one request with what its collection and folders set and the
/// merged env (`env_name` overrides the active one), persist the variables
/// its actions set and record it in history.
pub async fn send_and_record(
    client: &reqwest::Client,
    project: &Path,
    path: &ScenarioPath,
    scenario: &Scenario,
    env_name: Option<&str>,
) -> Result<Response> {
    let inherited = collection::defaults_chain(project, path.dir())?;
    let vars = env::resolve_for_request(project, env_name, &inherited, &scenario.runtime.variables)?;
    let response = http::send(client, project, scenario, &inherited, &vars).await?;
    if !response.captured.is_empty() {
        env::merge_captured(project, &response.captured)?;
    }
    record_history(project, path, &response);
    Ok(response)
}

/// History is a convenience; failing to write it shouldn't fail the request.
fn record_history(project: &Path, path: &ScenarioPath, response: &Response) {
    let entry = HistoryEntry { at: now_unix_millis(), response: response.clone() };
    let _ = history::push(project, path, entry);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RunResult {
    pub rel_path: ScenarioPath,
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

/// Run every request under `dir` (all when empty) in listing order, calling
/// `on_result` as each finishes. Variables set by earlier requests are
/// visible to later ones.
pub async fn run(
    client: &reqwest::Client,
    project: &Path,
    dir: &str,
    env_name: Option<&str>,
    mut on_result: impl FnMut(&RunResult),
) -> Result<Vec<RunResult>> {
    let entries = scenario::list(project)?.into_iter().filter(|entry| entry.rel_path.is_within(dir));
    let mut results = Vec::new();
    for entry in entries {
        let result = run_one(client, project, entry, env_name).await;
        on_result(&result);
        results.push(result);
    }
    Ok(results)
}

async fn run_one(client: &reqwest::Client, project: &Path, entry: ScenarioEntry, env_name: Option<&str>) -> RunResult {
    let scenario = match scenario::load(project, &entry.rel_path) {
        Ok(scenario) => scenario,
        Err(error) => return RunResult::errored(entry, &error),
    };
    match send_and_record(client, project, &entry.rel_path, &scenario, env_name).await {
        Ok(response) => RunResult::completed(entry, &scenario, response),
        Err(error) => RunResult::errored(entry, &error),
    }
}
