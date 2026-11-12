mod flow;
mod import;
mod run;
#[cfg(feature = "encryption")]
mod secrets;
mod send;

use cactus_core::env::Scope;
use cactus_core::http::build_client;

use crate::args::Command;
use crate::outcome::Outcome;

/// Without encryption a sealed secret fails at resolve, before anything is sent.
#[cfg(not(feature = "encryption"))]
mod password {
    #[allow(clippy::unnecessary_wraps)] // The same signature as with encryption.
    pub fn unlock_if_needed(_project: &std::path::Path, _password_stdin: bool) -> cactus_core::Result<()> {
        Ok(())
    }
}
#[cfg(feature = "encryption")]
use crate::password;

pub async fn execute(command: Command) -> cactus_core::Result<Outcome> {
    let client = build_client();
    match command {
        Command::Run { project, folder, env, json, critter, password_stdin } => {
            password::unlock_if_needed(&project, password_stdin)?;
            let scope = Scope::env(&project, env.as_deref()).with_critter(critter);
            run::run(&client, &project, &folder, &scope, Format::from_json_flag(json)).await
        }
        Command::Send { project, scenario, env, critter, password_stdin } => {
            password::unlock_if_needed(&project, password_stdin)?;
            let scope = Scope::env(&project, env.as_deref()).with_critter(critter);
            send::send(&client, &project, &scenario, &scope).await
        }
        Command::Flow { project, critter, flow, all, env, json, junit, password_stdin } => {
            password::unlock_if_needed(&project, password_stdin)?;
            let selection = match (critter, flow) {
                (Some(critter), Some(flow)) if !all => flow::Selection::One { critter, flow },
                _ => flow::Selection::All,
            };
            flow::flow(&client, &project, &selection, env, Format::from_json_flag(json), junit.as_deref()).await
        }
        Command::Import { project, file } => import::import(&project, &file),
        #[cfg(feature = "encryption")]
        Command::Secrets { project, action, password_stdin } => secrets::secrets(&project, action, password_stdin),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// One line per request as it finishes, then a summary.
    Report,
    /// A single JSON document at the end, for machines.
    Json,
}

impl Format {
    pub fn from_json_flag(json: bool) -> Self {
        if json { Self::Json } else { Self::Report }
    }
}
