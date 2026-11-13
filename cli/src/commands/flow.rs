//! `cactus-cli flow`: one flow, or all of them, with an optional `JUnit` report.

use std::fs;
use std::path::Path;

use cactus_core::runner::{self, FlowEvent, FlowRun, SendOptions};
use cactus_core::{Error, critter};
use serde_json::json;

use super::Format;
use crate::junit;
use crate::outcome::Outcome;
use crate::output::{format_flow_heading, format_flow_summary, format_step_result};

/// Which flows to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    One {
        critter: String,
        flow: String,
    },
    /// Every flow of every critter, critters and flows in name order.
    All,
}

/// Run the selected flows one after another. Fails if any step failed;
/// there is no scheduler here, so run `flow --all` from cron or CI.
pub async fn flow(
    client: &cactus_core::reqwest::Client,
    project: &Path,
    selection: &Selection,
    env: Option<String>,
    format: Format,
    junit_file: Option<&Path>,
) -> cactus_core::Result<Outcome> {
    let mut runs = Vec::new();
    for (critter, flow) in selected(project, selection) {
        if format == Format::Report {
            println!("{}", format_flow_heading(&critter, &flow));
        }
        let report = |event: FlowEvent<'_>| {
            if let (FlowEvent::Finished(step), Format::Report) = (event, format) {
                println!("{}", format_step_result(step));
            }
        };
        let run =
            runner::run_flow(client, project, &critter, &flow, env.clone(), SendOptions::INTERACTIVE, report).await?;
        runs.push((critter, flow, run));
    }
    match format {
        Format::Report => println!("{}", format_flow_summary(&runs)),
        Format::Json => {
            let report: Vec<_> =
                runs.iter().map(|(critter, flow, run)| json!({"critter": critter, "flow": flow, "run": run})).collect();
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    if let Some(file) = junit_file {
        fs::write(file, junit::report(&runs)).map_err(|source| Error::File { path: file.to_path_buf(), source })?;
    }
    Ok(Outcome::from_passed(runs.iter().all(|(_, _, run): &(String, String, FlowRun)| run.ok)))
}

fn selected(project: &Path, selection: &Selection) -> Vec<(String, String)> {
    match selection {
        Selection::One { critter, flow } => vec![(critter.clone(), flow.clone())],
        Selection::All => critter::list(project)
            .into_iter()
            .flat_map(|summary| summary.flows.into_iter().map(move |flow| (summary.id.clone(), flow.id)))
            .collect(),
    }
}
