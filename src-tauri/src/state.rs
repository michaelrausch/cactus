use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use cactus_core::http::build_client;
use cactus_core::reqwest;
use tokio::sync::oneshot;

use crate::watcher::ProjectWatcher;

/// Process-wide state shared by every command.
pub struct AppState {
    pub client: reqwest::Client,
    pub recent_projects_file: PathBuf,
    cancel: CancelSlot,
    /// Watches the open project; replaced when another project opens.
    watcher: Mutex<Option<ProjectWatcher>>,
}

impl AppState {
    pub fn new(recent_projects_file: PathBuf) -> Self {
        Self { client: build_client(), recent_projects_file, cancel: CancelSlot::default(), watcher: Mutex::default() }
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
