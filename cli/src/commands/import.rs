//! `cactus-cli import`: a Postman collection or OpenAPI spec into the project.

use std::path::Path;

use cactus_core::import;

use crate::outcome::Outcome;
use crate::output::format_import_summary;

pub fn import(project: &Path, file: &Path) -> cactus_core::Result<Outcome> {
    let summary = import::import(project, file)?;
    println!("{}", format_import_summary(&summary));
    Ok(Outcome::Passed)
}
