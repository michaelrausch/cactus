//! `cactus-cli send`: the body goes to stdout and everything else to stderr,
//! so the body can be piped into `jq` and the like.

use std::path::Path;

use cactus_core::env::Scope;
use cactus_core::http::scenario_passed;
use cactus_core::runner::SendOptions;
use cactus_core::{ProjectPath, runner, scenario};

use crate::outcome::Outcome;
use crate::output::format_status_line;

pub async fn send(
    client: &cactus_core::reqwest::Client,
    project: &Path,
    request_path: &str,
    scope: &Scope,
) -> cactus_core::Result<Outcome> {
    let path = ProjectPath::parse(request_path)?;
    let scenario = scenario::load(project, &path)?;
    let response = runner::send_as(client, project, &path, &scenario, scope, SendOptions::INTERACTIVE).await?;
    eprintln!("{}", format_status_line(&response));
    for warning in &response.warnings {
        eprintln!("warning: {warning}");
    }
    for failure in &response.capture_failures {
        eprintln!("warning: capture {} failed: {}", failure.name, failure.reason);
    }
    println!("{}", response.body);
    Ok(Outcome::from_passed(scenario_passed(&scenario, &response)))
}
