//! `cactus-cli secrets`: manage encryption without the app.

use std::path::Path;

use cactus_core::secrets::{self, DEFAULT_AUTO_LOCK_MINUTES, SecretsStatus};
use cactus_core::{Error, Result};

use crate::args::SecretsAction;
use crate::outcome::Outcome;
use crate::password::{self, PASSWORD_ENV};

/// `cactus-cli secrets <project> status|enable|disable|change-password`,
/// for headless machines.
pub fn secrets(project: &Path, action: SecretsAction, password_stdin: bool) -> Result<Outcome> {
    match action {
        SecretsAction::Status => {
            let info = secrets::info(project)?;
            let line = match (info.status, info.has_sealed_values) {
                (SecretsStatus::Off, _) => "off",
                (_, true) => "on",
                (_, false) => "on (no secrets yet)",
            };
            println!("{line}");
        }
        SecretsAction::Enable => {
            let password = password::read_new(password_stdin, "New secrets password: ")?;
            secrets::enable(project, &password, Some(DEFAULT_AUTO_LOCK_MINUTES))?;
            eprintln!("Secrets encrypted");
        }
        SecretsAction::Disable => {
            let password = password::read(password_stdin, "Secrets password: ")?;
            secrets::disable(project, &password)?;
            eprintln!("Encryption off");
        }
        SecretsAction::ChangePassword => {
            // One env var can't hold two passwords.
            if !password_stdin && std::env::var_os(PASSWORD_ENV).is_some() {
                return Err(Error::PasswordRequired);
            }
            let current = password::read(password_stdin, "Current password: ")?;
            let new = password::read_new(password_stdin, "New password: ")?;
            secrets::change_password(project, &current, &new)?;
            eprintln!("Password changed");
        }
    }
    Ok(Outcome::Passed)
}
