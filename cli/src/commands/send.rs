use std::path::Path;

use cactus_core::http::scenario_passed;
use cactus_core::{ScenarioPath, runner, scenario};

use crate::outcome::Outcome;
use crate::output::format_status_line;

pub async fn send(
    client: &cactus_core::reqwest::Client,
    project: &Path,
    scenario_path: &str,
    env: Option<&str>,
) -> cactus_core::Result<Outcome> {
    let path = ScenarioPath::parse(scenario_path)?;
    let scenario = scenario::load(project, &path)?;
    let response = runner::send_and_record(client, project, &path, &scenario, env).await?;
    eprintln!("{}", format_status_line(&response));
    println!("{}", response.body);
    Ok(Outcome::from_passed(scenario_passed(&scenario, &response)))
}
