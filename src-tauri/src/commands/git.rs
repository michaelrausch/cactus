//! Git: status, staging, diffs, commits and branches for the project's repo.

use std::path::PathBuf;

use cactus_core::git::{self, BranchInfo, CommitSummary, DiffSide, FileDiff, GitIdentity, GitStatus};
use cactus_core::{ProjectPath, Result};

/// `None` when the project isn't in a git repository.
#[tauri::command(async)]
#[specta::specta]
pub fn git_status(project: PathBuf) -> Result<Option<GitStatus>> {
    git::status(&project)
}

/// Make the project folder a git repository.
#[tauri::command(async)]
#[specta::specta]
pub fn git_init(project: PathBuf) -> Result<()> {
    git::init(&project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn git_stage(project: PathBuf, paths: Vec<ProjectPath>) -> Result<()> {
    git::stage(&project, &paths)
}

#[tauri::command(async)]
#[specta::specta]
pub fn git_unstage(project: PathBuf, paths: Vec<ProjectPath>) -> Result<()> {
    git::unstage(&project, &paths)
}

#[tauri::command(async)]
#[specta::specta]
pub fn git_stage_all(project: PathBuf) -> Result<()> {
    git::stage_all(&project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn git_unstage_all(project: PathBuf) -> Result<()> {
    git::unstage_all(&project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn git_discard(project: PathBuf, paths: Vec<ProjectPath>) -> Result<()> {
    git::discard(&project, &paths)
}

#[tauri::command(async)]
#[specta::specta]
pub fn git_discard_all(project: PathBuf) -> Result<()> {
    git::discard_all(&project)
}

#[tauri::command(async)]
#[specta::specta]
pub fn git_diff(project: PathBuf, path: ProjectPath, side: DiffSide) -> Result<FileDiff> {
    git::diff(&project, &path, side)
}

/// `author` overrides git config's identity when both parts are filled in.
#[tauri::command(async)]
#[specta::specta]
pub fn git_commit(project: PathBuf, message: String, author: Option<GitIdentity>) -> Result<CommitSummary> {
    git::commit(&project, &message, author.as_ref())
}

/// The identity git config gives, for the project's repository if one is
/// open; `None` when it isn't set.
#[tauri::command(async)]
#[specta::specta]
pub fn git_configured_identity(project: Option<PathBuf>) -> Result<Option<GitIdentity>> {
    git::configured_identity(project.as_deref())
}

#[tauri::command(async)]
#[specta::specta]
pub fn git_branches(project: PathBuf) -> Result<Vec<BranchInfo>> {
    git::branches(&project)
}

/// `checkout`: switch to the new branch too. (`switch` is reserved in TypeScript.)
#[tauri::command(async)]
#[specta::specta]
pub fn git_create_branch(project: PathBuf, name: String, checkout: bool) -> Result<()> {
    git::create_branch(&project, &name, checkout)
}

#[tauri::command(async)]
#[specta::specta]
pub fn git_switch_branch(project: PathBuf, name: String) -> Result<()> {
    git::switch_branch(&project, &name)
}

#[tauri::command(async)]
#[specta::specta]
pub fn git_delete_branch(project: PathBuf, name: String, force: bool) -> Result<()> {
    git::delete_branch(&project, &name, force)
}
