use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Headless runner for cactus projects, e.g. in CI.
#[derive(Debug, Parser)]
#[command(name = "cactus-cli", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run every scenario in the project, or in one folder of it.
    Run {
        project: PathBuf,
        /// Folder inside the project; the whole project when omitted.
        #[arg(default_value = "")]
        folder: String,
        /// Named environment to use instead of the active one.
        #[arg(short, long, value_name = "NAME")]
        env: Option<String>,
        /// Print results as JSON instead of a report.
        #[arg(long)]
        json: bool,
    },
    /// Send one scenario and print its response body.
    Send {
        project: PathBuf,
        /// Scenario path relative to the project, e.g. `auth/login.json`.
        scenario: String,
        #[arg(short, long, value_name = "NAME")]
        env: Option<String>,
    },
    /// Import a Postman collection or OpenAPI spec into the project.
    Import { project: PathBuf, file: PathBuf },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_with_flags_anywhere() {
        let cli = Cli::try_parse_from(["cactus-cli", "run", "--json", "proj", "auth", "-e", "dev"]).unwrap();
        let Command::Run { project, folder, env, json } = cli.command else { panic!("expected run") };
        assert_eq!((project, folder.as_str(), env.as_deref(), json), ("proj".into(), "auth", Some("dev"), true));
    }

    #[test]
    fn send_requires_a_scenario() {
        assert!(Cli::try_parse_from(["cactus-cli", "send", "proj"]).is_err());
    }
}
