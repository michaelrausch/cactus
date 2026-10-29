//! Events the backend pushes to the UI.

use cactus_core::runner::RunResult;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

/// Project-relative paths of files that changed on disk.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FsChanged(pub Vec<String>);

/// One scenario of a folder run finished.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct RunProgress(pub RunResult);
