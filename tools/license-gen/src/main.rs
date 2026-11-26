//! Issues cactus pro license keys. Never shipped: it needs the private
//! signing key, which lives outside the repository.
//!
//! ```text
//! license-gen keypair ~/secrets/cactus-signing.key   # once
//! license-gen issue --key ~/secrets/cactus-signing.key --name "Ada" --email ada@example.com
//! ```

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cactus_core::license::{self, License};
use clap::{Parser, Subcommand};
use data_encoding::HEXLOWER;

#[derive(Parser)]
#[command(about = "Issue cactus pro license keys")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Make a new signing key at PATH, and print its public half for `core/src/license.rs`.
    Keypair { path: PathBuf },
    /// Print a license key for NAME and EMAIL.
    Issue {
        /// The signing key `keypair` made.
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        email: String,
    },
}

fn main() -> ExitCode {
    let result = match Args::parse().command {
        Command::Keypair { path } => keypair(&path),
        Command::Issue { key, name, email } => issue(&key, name, email),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("license-gen: {error}");
            ExitCode::FAILURE
        }
    }
}

fn keypair(path: &Path) -> Result<(), String> {
    let mut signing_key = [0u8; 32];
    getrandom::fill(&mut signing_key).map_err(|e| e.to_string())?;
    // `create_new`: never overwrite a key that has issued licenses.
    let mut file =
        OpenOptions::new().write(true).create_new(true).open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    file.write_all(HEXLOWER.encode(&signing_key).as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
    let public = license::public_key(&signing_key);
    let bytes: Vec<String> = public.iter().map(|byte| format!("0x{byte:02x}")).collect();
    println!("Signing key written to {}. Keep it secret and backed up.", path.display());
    println!("In core/src/license.rs:\n\nconst PUBLIC_KEY: [u8; 32] = [{}];", bytes.join(", "));
    Ok(())
}

fn issue(key: &Path, name: String, email: String) -> Result<(), String> {
    let text = fs::read_to_string(key).map_err(|e| format!("{}: {e}", key.display()))?;
    let bytes = HEXLOWER.decode(text.trim().as_bytes()).map_err(|_| format!("{}: not a signing key", key.display()))?;
    let signing_key: [u8; 32] = bytes.try_into().map_err(|_| format!("{}: not a signing key", key.display()))?;
    let mut id = [0u8; 4];
    getrandom::fill(&mut id).map_err(|e| e.to_string())?;
    let license = License { id: HEXLOWER.encode(&id), name, email };
    let issued = license::issue(&license, &signing_key).map_err(|e| e.to_string())?;
    println!("{issued}");
    Ok(())
}
