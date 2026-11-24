//! Headless runner for cactus projects, e.g. in CI.

mod args;
mod commands;
mod junit;
mod outcome;
mod output;

use std::process::ExitCode;

use clap::Parser;

use crate::args::Cli;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    outcome::exit_code(commands::execute(cli.command).await)
}
