use std::path::Path;

use cactus_core::env::Scope;
use cactus_core::runner;

use super::Format;
use crate::outcome::Outcome;
use crate::output::{format_run_result, format_run_summary};

pub async fn run(
    client: &cactus_core::reqwest::Client,
    project: &Path,
    folder: &str,
    scope: &Scope,
    format: Format,
) -> cactus_core::Result<Outcome> {
    let results = runner::run(client, project, folder, scope, |result| {
        if format == Format::Report {
            println!("{}", format_run_result(result));
        }
    })
    .await?;
    match format {
        Format::Report => println!("{}", format_run_summary(&results)),
        Format::Json => println!("{}", serde_json::to_string_pretty(&results)?),
    }
    Ok(Outcome::from_passed(results.iter().all(|r| r.ok)))
}
