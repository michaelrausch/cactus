//! The HTTP client. Build one per process and share it, so connections are
//! reused across sends.

use std::sync::Arc;
use std::time::Duration;

use crate::cookies::ScopedJar;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT: &str = concat!("cactus/", env!("CARGO_PKG_VERSION"));

/// The shared client. Reuse one per process so connections are pooled.
///
/// # Panics
/// Only if the TLS backend cannot initialise, which is unrecoverable.
pub fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        // Cookies of the critter a send is made as; see `crate::cookies`.
        .cookie_provider(Arc::new(ScopedJar))
        .build()
        .expect("the HTTP client's static configuration is valid")
}
