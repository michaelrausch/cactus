//! One request sent as nobody and as every critter, to compare what each
//! persona gets back.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{SendOptions, send_as};
use crate::env::Scope;
use crate::http::Response;
use crate::scenario::Scenario;
use crate::{ProjectPath, Result, critter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MatrixCell {
    /// `None` is the row with no critter.
    pub critter: Option<String>,
    pub response: Option<Response>,
    pub error: Option<String>,
    /// Why the critter wasn't sent as, e.g. missing secrets.
    pub skipped: Option<String>,
}

/// Send `scenario` as no critter, then as each critter in turn, in the
/// active env, calling `on_cell` as each finishes. These are real sends:
/// captures go to each critter's context, but nothing goes into history.
/// Critters with missing secrets or an unreadable `critter.yml` are skipped.
pub async fn run_matrix(
    client: &reqwest::Client,
    project: &Path,
    path: &ProjectPath,
    scenario: &Scenario,
    mut on_cell: impl FnMut(&MatrixCell) + Send,
) -> Result<Vec<MatrixCell>> {
    let base = Scope::env(project, None);
    let critters = critter::list(project)?;
    let mut rows = vec![(None, None)];
    rows.extend(critters.into_iter().map(|summary| {
        let skipped = match (&summary.error, summary.missing_secrets.as_slice()) {
            (Some(error), _) => Some(error.clone()),
            (None, []) => None,
            (None, missing) => Some(format!("missing secrets: {}", missing.join(", "))),
        };
        (Some(summary.id), skipped)
    }));
    let mut cells = Vec::with_capacity(rows.len());
    for (critter, skipped) in rows {
        let cell = if let Some(reason) = skipped {
            MatrixCell { critter, response: None, error: None, skipped: Some(reason) }
        } else {
            let scope = base.clone().with_critter(critter.clone());
            match send_as(client, project, path, scenario, &scope, SendOptions::BACKGROUND).await {
                Ok(response) => MatrixCell { critter, response: Some(response), error: None, skipped: None },
                Err(error) => MatrixCell { critter, response: None, error: Some(error.to_string()), skipped: None },
            }
        };
        on_cell(&cell);
        cells.push(cell);
    }
    Ok(cells)
}
