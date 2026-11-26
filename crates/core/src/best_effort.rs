//! Work that must not fail its caller, but shouldn't vanish without a trace.

use std::fmt::Display;

/// Print `what` and the error to stderr when `result` failed. For writes like
/// history and run logs, where failing the request would be worse than
/// losing the record.
pub fn log<T, E: Display>(what: &str, result: Result<T, E>) {
    if let Err(error) = result {
        eprintln!("cactus: couldn't {what}: {error}");
    }
}
