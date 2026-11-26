//! How flow tools reach the person at the desktop app: questions become a
//! [`FlowAsked`] event answered by [`answer_flow_ask`], notifications a
//! [`FlowNotified`] event, and URLs open in the default browser.
//!
//! [`answer_flow_ask`]: crate::commands::flow::answer_flow_ask

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use cactus_core::env::Env;
use cactus_core::tools::{Answer, Ask, BoxFuture, ToolHost};
use cactus_core::{Error, Result};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::oneshot;

use crate::events::{FlowAskClosed, FlowAsked, FlowDisplayed, FlowNotified, FlowToolProgress, notify};
use crate::state::AppState;

/// Questions waiting for an answer from the UI, by id.
#[derive(Default)]
pub struct PendingAsks {
    next: AtomicU32,
    waiting: Mutex<HashMap<u32, oneshot::Sender<Option<Env>>>>,
}

impl PendingAsks {
    fn open(&self) -> (u32, oneshot::Receiver<Option<Env>>) {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let (answer, answered) = oneshot::channel();
        self.waiting.lock().unwrap_or_else(PoisonError::into_inner).insert(id, answer);
        (id, answered)
    }

    /// Answer question `id`: its values, or `None` to stop the run. An
    /// unknown id (the run was cancelled meanwhile) is ignored.
    pub fn answer(&self, id: u32, values: Option<Env>) {
        if let Some(waiting) = self.waiting.lock().unwrap_or_else(PoisonError::into_inner).remove(&id) {
            let _ = waiting.send(values);
        }
    }

    /// Forget `id`; whether it was still waiting.
    fn close(&self, id: u32) -> bool {
        self.waiting.lock().unwrap_or_else(PoisonError::into_inner).remove(&id).is_some()
    }
}

/// Forgets a question when its run stops waiting, e.g. because it was
/// cancelled, and tells the UI to close it.
struct Closing<'a> {
    app: &'a AppHandle,
    id: u32,
}

impl Drop for Closing<'_> {
    fn drop(&mut self) {
        if self.app.state::<AppState>().asks.close(self.id) {
            notify(&FlowAskClosed { id: self.id }, self.app);
        }
    }
}

/// The desktop app running a flow. A run the person started can ask and
/// open pages; a scheduled one can only notify.
pub struct DesktopHost {
    app: AppHandle,
    critter: String,
    flow: String,
    interactive: bool,
    /// When progress was last sent, to send at most every [`PROGRESS_EVERY`].
    progressed: Mutex<Option<Instant>>,
}

const PROGRESS_EVERY: Duration = Duration::from_millis(100);

impl DesktopHost {
    pub fn new(app: AppHandle, critter: String, flow: String, interactive: bool) -> Self {
        Self { app, critter, flow, interactive, progressed: Mutex::new(None) }
    }

    /// Whether to send progress now: always the last update, otherwise at
    /// most every [`PROGRESS_EVERY`].
    fn progress_due(&self, done: u32, total: u32) -> bool {
        let mut progressed = self.progressed.lock().unwrap_or_else(PoisonError::into_inner);
        let now = Instant::now();
        let finished = done >= total;
        let recently = progressed.is_some_and(|at| now.duration_since(at) < PROGRESS_EVERY);
        if !finished && recently {
            return false;
        }
        *progressed = Some(now);
        true
    }
}

impl ToolHost for DesktopHost {
    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer> {
        Box::pin(async move {
            if !self.interactive {
                return Answer::Unattended;
            }
            let (id, answered) = self.app.state::<AppState>().asks.open();
            let _closing = Closing { app: &self.app, id };
            let (critter, flow) = (self.critter.clone(), self.flow.clone());
            notify(&FlowAsked { id, critter, flow, ask }, &self.app);
            match answered.await {
                Ok(Some(values)) => Answer::Given(values),
                Ok(None) | Err(_) => Answer::Stopped,
            }
        })
    }

    fn show(&self, title: &str, text: &str) {
        // A schedule's messages would pop up out of nowhere; its run log has them.
        if !self.interactive {
            return;
        }
        let (critter, flow) = (self.critter.clone(), self.flow.clone());
        notify(&FlowDisplayed { critter, flow, title: title.into(), text: text.into() }, &self.app);
    }

    fn notify(&self, title: &str, body: &str) {
        let (critter, flow) = (self.critter.clone(), self.flow.clone());
        notify(&FlowNotified { critter, flow, title: title.into(), body: body.into() }, &self.app);
    }

    fn open_url(&self, url: &str) -> Result<()> {
        // A schedule mustn't pop pages up while you're doing something else.
        if !self.interactive {
            return Ok(());
        }
        self.app.opener().open_url(url, None::<&str>).map_err(|e| Error::Tool(format!("Couldn't open {url}: {e}")))
    }

    fn progress(&self, index: u32, done: u32, total: u32) {
        // Without a total there's nothing to show.
        if !self.interactive || total == 0 || !self.progress_due(done, total) {
            return;
        }
        let (critter, flow) = (self.critter.clone(), self.flow.clone());
        notify(&FlowToolProgress { critter, flow, index, done, total }, &self.app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn answers_reach_their_question_once() {
        let asks = PendingAsks::default();
        let (first, answered) = asks.open();
        let (second, _) = asks.open();
        assert_ne!(first, second);
        asks.answer(first, Some(Env::from([("code".into(), "1".into())])));
        asks.answer(first, None);
        assert_eq!(answered.await.unwrap().unwrap()["code"], "1");
        assert!(asks.close(second) && !asks.close(second));
    }
}
