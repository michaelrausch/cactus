//! Whether usecactus.dev has a newer release than the one running.

use serde::Deserialize;

use crate::Result;

/// The download buttons on the landing page.
pub const DOWNLOAD_PAGE: &str = "https://usecactus.dev/#download";

/// The latest released version, as `{ "version": "1.2.0" }`.
const VERSION_URL: &str = "https://usecactus.dev/version.json";

#[derive(Deserialize)]
struct VersionFile {
    version: String,
}

/// The website's latest version, if it is newer than `current`.
pub async fn check(client: &reqwest::Client, current: &str) -> Result<Option<String>> {
    let body = client.get(VERSION_URL).send().await?.error_for_status()?.text().await?;
    let latest = serde_json::from_str::<VersionFile>(&body)?.version;
    Ok(is_newer(&latest, current).then_some(latest))
}

/// Whether `latest` is a higher `major.minor.patch` than `current`. Anything
/// that doesn't parse is never newer, so a bad file can't nag.
pub fn is_newer(latest: &str, current: &str) -> bool {
    matches!((parse(latest), parse(current)), (Some(latest), Some(current)) if latest > current)
}

/// `1.2.0`, `v1.2` or `1.2.0-beta` as `(1, 2, 0)`: missing parts are 0, and
/// pre-release or build suffixes are ignored.
fn parse(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.trim().trim_start_matches('v').split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let mut next = || parts.next().map_or(Some(0), |part| part.parse().ok());
    let parsed = (next()?, next()?, next()?);
    parts.next().is_none().then_some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_numerically() {
        assert!(is_newer("1.2.0", "0.1.0"));
        assert!(is_newer("1.10.0", "1.9.9"));
        assert!(is_newer("1.2.1", "1.2.0"));
        assert!(!is_newer("1.2.0", "1.2.0"));
        assert!(!is_newer("1.1.9", "1.2.0"));
    }

    #[test]
    fn reads_loose_versions() {
        assert!(!is_newer("v1.2", "1.2.0"));
        assert!(is_newer("1.3", "1.2.9"));
        assert!(!is_newer("1.2.0-beta", "1.2.0"));
    }

    #[test]
    fn junk_is_never_newer() {
        assert!(!is_newer("", "1.2.0"));
        assert!(!is_newer("latest", "1.2.0"));
        assert!(!is_newer("2.0.0.0", "1.2.0"));
        assert!(!is_newer("2.0.0", "nonsense"));
    }
}
