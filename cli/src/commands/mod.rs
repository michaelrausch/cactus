//! One module per subcommand; [`execute`] dispatches.

mod flow;
mod import;
mod run;
mod send;

use std::io::IsTerminal;
use std::path::Path;

use cactus_core::env::Scope;
use cactus_core::http::build_client;
use cactus_core::secrets::{self, Unlocked, VaultStatus};

use crate::args::Command;
use crate::outcome::Outcome;
use crate::tool_host::CliHost;

/// Where a locked project's password comes from when there's no one to ask.
const PASSWORD_VAR: &str = "CACTUS_PASSWORD";
/// And its authenticator code, with TOTP on.
const CODE_VAR: &str = "CACTUS_TOTP_CODE";

pub async fn execute(command: Command) -> cactus_core::Result<Outcome> {
    unlock(command.project())?;
    let client = build_client();
    match command {
        Command::Run { project, folder, env, json, critter } => {
            let scope = Scope::env(&project, env.as_deref()).with_critter(critter);
            run::run(&client, &project, &folder, &scope, Format::from_json_flag(json)).await
        }
        Command::Send { project, scenario, env, critter } => {
            let scope = Scope::env(&project, env.as_deref()).with_critter(critter);
            send::send(&client, &project, &scenario, &scope).await
        }
        Command::Flow { project, critter, flow, all, env, json, junit, vars } => {
            let selection = match (critter, flow) {
                (Some(critter), Some(flow)) if !all => flow::Selection::One { critter, flow },
                _ => flow::Selection::All,
            };
            let host = CliHost::new(vars.into_iter().collect());
            let format = Format::from_json_flag(json);
            flow::flow(&client, &project, &selection, env, format, junit.as_deref(), &host).await
        }
        Command::Import { project, file } => import::import(&project, &file),
    }
}

/// With a password set, take it from `CACTUS_PASSWORD` or ask once.
fn unlock(project: &Path) -> cactus_core::Result<()> {
    if secrets::status(project) != VaultStatus::Locked {
        return Ok(());
    }
    let password = from_env_or_ask(PASSWORD_VAR, "Password: ")?;
    if secrets::unlock(project, &password, None)? == Unlocked::CodeRequired {
        let code = from_env_or_ask(CODE_VAR, "Authenticator code: ")?;
        secrets::unlock(project, &password, Some(&code))?;
    }
    Ok(())
}

/// `var`, or typed in (hidden) at a terminal; with neither, the project stays locked.
fn from_env_or_ask(var: &str, prompt: &str) -> cactus_core::Result<String> {
    match std::env::var(var) {
        Ok(value) => Ok(value),
        Err(_) if std::io::stdin().is_terminal() => Ok(rpassword::prompt_password(prompt)?),
        Err(_) => Err(cactus_core::Error::Locked),
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
