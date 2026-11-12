//! Where the CLI gets the secrets password, in order: `--password-stdin`
//! (one line, for scripts), `CACTUS_SECRETS_PASSWORD` (for CI), a prompt
//! when stdin is a terminal. Otherwise it fails before anything is sent.

use std::io::{self, BufRead, IsTerminal};
use std::path::Path;

use cactus_core::secrets::{self, SecretString};
use cactus_core::{Error, Result};

pub const PASSWORD_ENV: &str = "CACTUS_SECRETS_PASSWORD";

/// Where a password can come from on this run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Stdin,
    Env,
    Prompt,
}

/// The first source available: the flag, then the env var, then a TTY.
pub fn source(password_stdin: bool, env_set: bool, stdin_is_tty: bool) -> Option<Source> {
    if password_stdin {
        Some(Source::Stdin)
    } else if env_set {
        Some(Source::Env)
    } else if stdin_is_tty {
        Some(Source::Prompt)
    } else {
        None
    }
}

/// If the project has sealed secrets, unlock them for this process without
/// writing anything. A wrong password fails here, before any send.
pub fn unlock_if_needed(project: &Path, password_stdin: bool) -> Result<()> {
    if !secrets::has_sealed_values(project)? {
        return Ok(());
    }
    let password = read(password_stdin, "Secrets password: ")?;
    secrets::unlock_read_only(project, &password)
}

/// One password from the first available source.
pub fn read(password_stdin: bool, prompt: &str) -> Result<SecretString> {
    let env = std::env::var(PASSWORD_ENV).ok();
    match source(password_stdin, env.is_some(), io::stdin().is_terminal()) {
        Some(Source::Stdin) => read_stdin_line(),
        Some(Source::Env) => Ok(SecretString::new(env.unwrap_or_default())),
        Some(Source::Prompt) => Ok(SecretString::new(rpassword::prompt_password(prompt)?)),
        None => Err(Error::PasswordRequired),
    }
}

/// A new password: asked twice at a prompt, once from other sources.
pub fn read_new(password_stdin: bool, prompt: &str) -> Result<SecretString> {
    let env_set = std::env::var_os(PASSWORD_ENV).is_some();
    if source(password_stdin, env_set, io::stdin().is_terminal()) != Some(Source::Prompt) {
        return read(password_stdin, prompt);
    }
    let first = rpassword::prompt_password(prompt)?;
    let again = rpassword::prompt_password("Again: ")?;
    if first != again {
        return Err(Error::Io(io::Error::other("the passwords don't match")));
    }
    Ok(SecretString::new(first))
}

pub fn read_stdin_line() -> Result<SecretString> {
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim_end_matches(['\n', '\r']).len();
    line.truncate(trimmed);
    Ok(SecretString::new(line))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources_in_order() {
        assert_eq!(source(true, true, true), Some(Source::Stdin));
        assert_eq!(source(false, true, true), Some(Source::Env));
        assert_eq!(source(false, false, true), Some(Source::Prompt));
        assert_eq!(source(false, false, false), None);
    }
}
