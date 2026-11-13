//! Command-line arguments. Doc comments here are the `--help` text.

use std::path::PathBuf;

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
        /// Read the secrets password from stdin (one line).
        #[arg(long, hide = !cactus_core::secrets::ENABLED)]
        password_stdin: bool,
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
        /// Read the secrets password from stdin (one line).
        #[arg(long, hide = !cactus_core::secrets::ENABLED)]
        password_stdin: bool,
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
        /// Read the secrets password from stdin (one line).
        #[arg(long, hide = !cactus_core::secrets::ENABLED)]
        password_stdin: bool,
    },
    /// Import a Postman collection or OpenAPI spec into the project.
    Import { project: PathBuf, file: PathBuf },
    /// Turn secret encryption on or off, or change its password.
    #[cfg(feature = "encryption")]
    Secrets {
        project: PathBuf,
        #[command(subcommand)]
        action: SecretsAction,
        /// Read passwords from stdin, one per line, instead of prompting.
        #[arg(long, global = true)]
        password_stdin: bool,
    },
}

/// What `secrets` does.
#[cfg(feature = "encryption")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum SecretsAction {
    /// Whether secrets are encrypted.
    Status,
    /// Encrypt every secret with a new password.
    Enable,
    /// Decrypt every secret back to plain text.
    Disable,
    /// Encrypt every secret again under a new password (current, then new).
    ChangePassword,
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
    fn sends_take_password_stdin() {
        let run = Cli::try_parse_from(["cactus-cli", "run", "proj", "--password-stdin"]).unwrap();
        assert!(matches!(run.command, Command::Run { password_stdin: true, .. }));
        let send = Cli::try_parse_from(["cactus-cli", "send", "proj", "me.yml"]).unwrap();
        assert!(matches!(send.command, Command::Send { password_stdin: false, .. }));
        let flow = Cli::try_parse_from(["cactus-cli", "flow", "proj", "--all", "--password-stdin"]).unwrap();
        assert!(matches!(flow.command, Command::Flow { password_stdin: true, .. }));
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn secrets_subcommands() {
        let parse = |args: &[&str]| Cli::try_parse_from([&["cactus-cli", "secrets", "proj"], args].concat());
        let enable = parse(&["enable", "--password-stdin"]).unwrap();
        assert!(matches!(enable.command, Command::Secrets { action: SecretsAction::Enable, password_stdin: true, .. }));
        let change = parse(&["change-password"]).unwrap();
        assert!(matches!(change.command, Command::Secrets { action: SecretsAction::ChangePassword, .. }));
        assert!(matches!(parse(&["status"]).unwrap().command, Command::Secrets { action: SecretsAction::Status, .. }));
        assert!(matches!(
            parse(&["disable"]).unwrap().command,
            Command::Secrets { action: SecretsAction::Disable, .. }
        ));
        assert!(parse(&["unknown"]).is_err());
        assert!(parse(&[]).is_err());
    }
}
