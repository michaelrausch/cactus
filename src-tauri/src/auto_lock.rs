//! Locks the open project's secrets after they've gone unused for its
//! auto-lock time. In the backend, because the key lives here and a hidden
//! webview's timers can be throttled.

use std::time::{Duration, Instant};

use cactus_core::secrets::{self, SecretsStatus};
use tauri::{AppHandle, Manager};

use crate::commands::secrets::status_changed;
use crate::state::AppState;

const CHECK_EVERY: Duration = Duration::from_secs(60);

/// Whether secrets last used at `last_activity` should be locked at `now`;
/// `minutes` of `None` is never.
pub fn should_lock(last_activity: Instant, now: Instant, minutes: Option<u32>) -> bool {
    minutes.is_some_and(|minutes| {
        now.saturating_duration_since(last_activity) >= Duration::from_secs(u64::from(minutes) * 60)
    })
}

/// Check once a minute for the life of the app.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(CHECK_EVERY);
        loop {
            tick.tick().await;
            check(&app);
        }
    });
}

fn check(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(project) = state.project() else { return };
    let Ok(info) = secrets::info(&project) else { return };
    if info.status == SecretsStatus::Unlocked
        && should_lock(state.secrets_activity(), Instant::now(), info.auto_lock_minutes)
    {
        secrets::lock(&project);
        status_changed(app, &project);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locks_after_the_idle_time_and_never_without_one() {
        let start = Instant::now();
        let after = |minutes: u64| start + Duration::from_secs(minutes * 60);
        assert!(!should_lock(start, after(14), Some(15)));
        assert!(should_lock(start, after(15), Some(15)));
        assert!(!should_lock(start, after(10_000), None));
        assert!(!should_lock(after(5), start, Some(1)), "a clock going backwards never locks");
    }
}
