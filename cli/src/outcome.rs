//! Pass/fail verdicts and the exit codes CI scripts rely on.

use std::process::ExitCode;

/// Exit code for bad usage or an error before any verdict (clap uses 2 too).
const EXIT_ERROR: u8 = 2;

/// The verdict of a command that ran to completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    Failed,
}

impl Outcome {
    pub fn from_passed(passed: bool) -> Self {
        if passed { Self::Passed } else { Self::Failed }
    }
}

/// 0 when everything passed, 1 when something failed, 2 on error.
pub fn exit_code(result: cactus_core::Result<Outcome>) -> ExitCode {
    match result {
        Ok(Outcome::Passed) => ExitCode::SUCCESS,
        Ok(Outcome::Failed) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}
