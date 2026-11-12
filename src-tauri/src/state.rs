use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use cactus_core::http::build_client;
use cactus_core::reqwest;
use tokio::sync::oneshot;

use crate::scheduler::Scheduler;
use crate::watcher::ProjectWatcher;

/// Process-wide state shared by every command.
pub struct AppState {
    pub client: reqwest::Client,
    pub recent_projects_file: PathBuf,
    /// Runs the open project's enabled flows on their timers.
    pub scheduler: Scheduler,
    cancel: CancelSlot,
    /// Watches the open project; replaced when another project opens.
    watcher: Mutex<Option<ProjectWatcher>>,
    /// The open project, which auto-lock watches.
    project: Mutex<Option<PathBuf>>,
    /// When secret values were last used interactively. Scheduled runs
    /// don't count, so a schedule can't keep secrets unlocked forever.
    secrets_activity: Mutex<Instant>,
}

impl AppState {
    pub fn new(recent_projects_file: PathBuf) -> Self {
        Self {
            client: build_client(),
            recent_projects_file,
            scheduler: Scheduler::default(),
            cancel: CancelSlot::default(),
            watcher: Mutex::default(),
            project: Mutex::default(),
            secrets_activity: Mutex::new(Instant::now()),
        }
    }

    pub fn set_project(&self, project: &Path) {
        *lock(&self.project) = Some(project.to_path_buf());
    }

    pub fn project(&self) -> Option<PathBuf> {
        lock(&self.project).clone()
    }

    /// Secret values were just used by something the user did.
    pub fn touch_secrets(&self) {
        *lock(&self.secrets_activity) = Instant::now();
    }

    pub fn secrets_activity(&self) -> Instant {
        *lock(&self.secrets_activity)
    }

    pub fn replace_watcher(&self, watcher: Option<ProjectWatcher>) {
        *lock(&self.watcher) = watcher;
    }

    /// Run `work` as the current cancellable operation, cancelling any
    /// previous one. Resolves to `None` if it is cancelled.
    pub async fn run_cancellable<T>(&self, work: impl Future<Output = T>) -> Option<T> {
        let cancelled = self.cancel.begin();
        tokio::select! {
            output = work => Some(output),
            _ = cancelled => None,
        }
    }

    pub fn cancel_current(&self) {
        self.cancel.cancel();
    }
}

/// Holds the trigger for the one operation that can be cancelled at a time.
#[derive(Default)]
struct CancelSlot(Mutex<Option<oneshot::Sender<()>>>);

impl CancelSlot {
    fn begin(&self) -> oneshot::Receiver<()> {
        let (trigger, cancelled) = oneshot::channel();
        if let Some(previous) = lock(&self.0).replace(trigger) {
            let _ = previous.send(());
        }
        cancelled
    }

    fn cancel(&self) {
        if let Some(trigger) = lock(&self.0).take() {
            let _ = trigger.send(());
        }
    }
}

/// The guarded values stay valid even if a holder panicked, so poisoning is ignored.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn starting_a_new_operation_cancels_the_previous_one() {
        let slot = CancelSlot::default();
        let first = slot.begin();
        let _second = slot.begin();
        assert!(first.await.is_ok());
    }
}
