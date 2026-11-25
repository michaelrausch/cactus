//! Command-line arguments. Doc comments here are the `--help` text.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

/// Headless runner for cactus projects (OpenCollection folders), e.g. in CI.
#[derive(Debug, Parser)]
#[command(name = "cactus-cli", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run every request in the project, or in one folder of it, in order.
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
        /// Send as this critter: its variables, secrets and context.
        #[arg(long = "as", value_name = "CRITTER")]
        critter: Option<String>,
    },
    /// Send one request and print its response body.
    Send {
        project: PathBuf,
        /// Request file relative to the project, e.g. `auth/login.yml`.
        scenario: String,
        #[arg(short, long, value_name = "NAME")]
        env: Option<String>,
        /// Send as this critter: its variables, secrets and context.
        #[arg(long = "as", value_name = "CRITTER")]
        critter: Option<String>,
    },
    /// Run a critter's flow, or with --all every flow of every critter.
    Flow {
        project: PathBuf,
        /// The critter whose flow to run, e.g. `bob`.
        #[arg(required_unless_present = "all", conflicts_with = "all")]
        critter: Option<String>,
        /// The flow, e.g. `health` for `critters/bob/flows/health.yml`.
        #[arg(required_unless_present = "all", conflicts_with = "all")]
        flow: Option<String>,
        /// Run every flow of every critter.
        #[arg(long)]
        all: bool,
        /// Named environment to use instead of the active one.
        #[arg(short, long, value_name = "NAME")]
        env: Option<String>,
        /// Print results as JSON instead of a report.
        #[arg(long)]
        json: bool,
        /// Also write a `JUnit` XML report to this file.
        #[arg(long, value_name = "FILE")]
        junit: Option<PathBuf>,
        /// Answer a prompt step's `name`, e.g. `--var code=123`; repeatable.
        #[arg(long = "var", value_name = "NAME=VALUE", value_parser = crate::tool_host::parse_var)]
        vars: Vec<(String, String)>,
    },
    /// Import a Postman collection or OpenAPI spec into the project.
    Import { project: PathBuf, file: PathBuf },
}

impl Command {
    pub fn project(&self) -> &Path {
        match self {
            Self::Run { project, .. }
            | Self::Send { project, .. }
            | Self::Flow { project, .. }
            | Self::Import { project, .. } => project,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_with_flags_anywhere() {
        let cli = Cli::try_parse_from(["cactus-cli", "run", "--json", "proj", "auth", "-e", "dev"]).unwrap();
        let Command::Run { project, folder, env, json, critter, .. } = cli.command else { panic!("expected run") };
        assert_eq!((project, folder.as_str(), env.as_deref(), json), ("proj".into(), "auth", Some("dev"), true));
        assert_eq!(critter, None);
    }

    #[test]
    fn send_requires_a_scenario() {
        assert!(Cli::try_parse_from(["cactus-cli", "send", "proj"]).is_err());
    }

    #[test]
    fn run_and_send_take_a_critter() {
        let run = Cli::try_parse_from(["cactus-cli", "run", "proj", "--as", "bob"]).unwrap();
        assert!(matches!(run.command, Command::Run { critter: Some(c), .. } if c == "bob"));
        let send = Cli::try_parse_from(["cactus-cli", "send", "--as", "bob", "proj", "me.yml"]).unwrap();
        assert!(matches!(send.command, Command::Send { critter: Some(c), .. } if c == "bob"));
    }

    #[test]
    fn flow_takes_a_critter_and_flow_or_all() {
        let one = Cli::try_parse_from(["cactus-cli", "flow", "proj", "bob", "health", "--junit", "out.xml"]).unwrap();
        let Command::Flow { critter, flow, all, junit, .. } = one.command else { panic!("expected flow") };
        assert_eq!((critter.as_deref(), flow.as_deref(), all), (Some("bob"), Some("health"), false));
        assert_eq!(junit, Some("out.xml".into()));
        let every = Cli::try_parse_from(["cactus-cli", "flow", "proj", "--all", "-e", "staging"]).unwrap();
        assert!(matches!(every.command, Command::Flow { all: true, critter: None, .. }));
        assert!(Cli::try_parse_from(["cactus-cli", "flow", "proj", "bob"]).is_err());
        assert!(Cli::try_parse_from(["cactus-cli", "flow", "proj"]).is_err());
        assert!(Cli::try_parse_from(["cactus-cli", "flow", "proj", "bob", "health", "--all"]).is_err());
    }

    #[test]
    fn flow_takes_vars_for_prompts() {
        let args = ["cactus-cli", "flow", "proj", "bob", "login", "--var", "code=123", "--var", "otp=9"];
        let Command::Flow { vars, .. } = Cli::try_parse_from(args).unwrap().command else { panic!("expected flow") };
        assert_eq!(vars, [("code".into(), "123".into()), ("otp".into(), "9".into())]);
        assert!(Cli::try_parse_from(["cactus-cli", "flow", "proj", "--all", "--var", "nope"]).is_err());
    }
}
