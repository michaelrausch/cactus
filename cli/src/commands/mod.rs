mod import;
mod run;
mod send;

use cactus_core::http::build_client;

use crate::args::Command;
use crate::outcome::Outcome;

pub async fn execute(command: Command) -> cactus_core::Result<Outcome> {
    let client = build_client();
    match command {
        Command::Run { project, folder, env, json } => {
            run::run(&client, &project, &folder, env.as_deref(), run::Format::from_json_flag(json)).await
        }
        Command::Send { project, scenario, env } => send::send(&client, &project, &scenario, env.as_deref()).await,
        Command::Import { project, file } => import::import(&project, &file),
    }
}
