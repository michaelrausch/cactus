//! The MCP server: lets agents (Claude Code and the like) list, read, send
//! and run the open project's requests and flows over Streamable HTTP on
//! `127.0.0.1`. It runs while the app is open and the setting is on, and
//! works on whatever project the app has open.

mod server;
mod settings;
mod tools;

use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use cactus_core::tools::ToolHost;
use cactus_core::{Result, best_effort};
use serde::Serialize;
use specta::Type;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use settings::McpSettings;
pub use settings::new_token;
use tools::{Change, McpApp};

use crate::events::{ContextChanged, HistoryChanged, notify};
use crate::state::AppState;
use crate::tool_host::DesktopHost;

/// What the settings tab shows.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub enabled: bool,
    pub allow_edits: bool,
    pub port: u16,
    pub token: String,
    pub url: String,
    pub running: bool,
    /// Why it isn't running though enabled, e.g. the port is taken.
    pub error: Option<String>,
}

/// Owned by [`AppState`]: the settings and the running server, if any.
pub struct Mcp {
    file: PathBuf,
    inner: Mutex<Inner>,
}

struct Inner {
    settings: McpSettings,
    running: Option<Running>,
    error: Option<String>,
}

struct Running {
    stop: CancellationToken,
    task: JoinHandle<()>,
}

impl Running {
    fn stop(self) {
        self.stop.cancel();
        // Not waiting for open streams to end frees the port at once.
        self.task.abort();
    }
}

impl Mcp {
    pub fn new(file: PathBuf) -> Self {
        let settings = McpSettings::load(&file);
        Self { file, inner: Mutex::new(Inner { settings, running: None, error: None }) }
    }

    pub async fn status(&self) -> McpStatus {
        let inner = self.inner.lock().await;
        let settings = &inner.settings;
        McpStatus {
            enabled: settings.enabled,
            allow_edits: settings.allow_edits,
            port: settings.port,
            token: settings.token.clone(),
            url: format!("http://127.0.0.1:{}/mcp", settings.port),
            running: inner.running.is_some(),
            error: inner.error.clone(),
        }
    }

    /// Change the settings, save them and restart the server to match.
    /// Connected agents reconnect, picking up the new token and tools.
    pub async fn update(&self, app: &AppHandle, change: impl FnOnce(&mut McpSettings)) -> Result<McpStatus> {
        {
            let mut inner = self.inner.lock().await;
            change(&mut inner.settings);
            inner.settings.save(&self.file)?;
            restart(app, &mut inner).await;
        }
        Ok(self.status().await)
    }

    /// Start the server if it's enabled; at launch.
    pub async fn start(&self, app: &AppHandle) {
        restart(app, &mut *self.inner.lock().await).await;
    }

    pub fn stop(&self) {
        if let Ok(mut inner) = self.inner.try_lock()
            && let Some(running) = inner.running.take()
        {
            running.stop();
        }
    }
}

async fn restart(app: &AppHandle, inner: &mut Inner) {
    if let Some(running) = inner.running.take() {
        running.stop();
    }
    inner.error = None;
    if !inner.settings.enabled {
        return;
    }
    let port = inner.settings.port;
    let listener = match bind(port).await {
        Ok(listener) => listener,
        Err(error) => {
            inner.error = Some(match error.kind() {
                ErrorKind::AddrInUse => format!("Port {port} is in use"),
                _ => error.to_string(),
            });
            return;
        }
    };
    let stop = CancellationToken::new();
    let desktop: Arc<dyn McpApp> = Arc::new(Desktop { app: app.clone() });
    let serving =
        server::serve(listener, desktop, inner.settings.allow_edits, inner.settings.token.clone(), stop.clone());
    let task = tauri::async_runtime::spawn(async move {
        best_effort::log("serve MCP", serving.await);
    });
    inner.running = Some(Running { stop, task });
}

/// Bind on loopback. A server just stopped may hold the port for a moment.
async fn bind(port: u16) -> std::io::Result<tokio::net::TcpListener> {
    let mut tries = 0;
    loop {
        match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            Err(error) if error.kind() == ErrorKind::AddrInUse && tries < 10 => {
                tries += 1;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            result => return result,
        }
    }
}

/// The tools' view of the desktop app.
struct Desktop {
    app: AppHandle,
}

impl McpApp for Desktop {
    fn project(&self) -> Option<PathBuf> {
        self.app.state::<AppState>().project()
    }

    fn client(&self) -> cactus_core::reqwest::Client {
        self.app.state::<AppState>().client.clone()
    }

    fn host(&self, critter: &str, flow: &str) -> Box<dyn ToolHost> {
        let (critter, flow) = (critter.to_string(), flow.to_string());
        Box::new(DesktopHost::new(self.app.clone(), critter, flow, true))
    }

    fn changed(&self, change: Change) {
        match change {
            Change::Context(critter) => notify(&ContextChanged { critter }, &self.app),
            Change::History(path) => notify(&HistoryChanged { path }, &self.app),
            Change::Flows => {
                let state = self.app.state::<AppState>();
                if let Some(project) = state.project() {
                    state.scheduler.reload(&self.app, &project);
                }
            }
        }
    }
}
