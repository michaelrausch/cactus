use std::path::Path;

use cactus_core::runner;

use crate::outcome::Outcome;
use crate::output::{format_run_result, format_run_summary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// One line per scenario as it finishes, then a summary.
    Report,
    /// A single JSON array at the end, for machines.
    Json,
}

impl Format {
    pub fn from_json_flag(json: bool) -> Self {
        if json { Self::Json } else { Self::Report }
    }
}

pub async fn run(
    client: &cactus_core::reqwest::Client,
    project: &Path,
    folder: &str,
    env: Option<&str>,
    format: Format,
) -> cactus_core::Result<Outcome> {
    let results = runner::run(client, project, folder, env, |result| {
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
