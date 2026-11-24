//! Values captured from responses: `.cactus/captured.json` with no critter,
//! a critter's `context.json` otherwise.
//!
//! A scheduled run and an interactive send can capture at the same moment,
//! so every read-modify-write of either file holds [`CONTEXT_WRITE`]; without
//! it one would lose the other's values. Another process (the CLI) writing at
//! the same time isn't covered.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::layers::Env;
use crate::clock::now_unix_millis;
use crate::critter::{ContextEntry, validate_critter_id};
use crate::storage::{read_json, write_json};
use crate::{Error, ProjectPath, Result, layout};

/// What a context entry edited by hand says it came from.
pub const EDITED: &str = "edited";

static CONTEXT_WRITE: Mutex<()> = Mutex::new(());

/// The files stay valid even if a holder panicked, so poisoning is ignored.
fn write_lock() -> MutexGuard<'static, ()> {
    CONTEXT_WRITE.lock().unwrap_or_else(PoisonError::into_inner)
}

pub type Context = BTreeMap<String, ContextEntry>;

/// Merge values set by actions into `.cactus/captured.json`.
pub fn merge_captured(project: &Path, values: &Env) -> Result<()> {
    let _guard = write_lock();
    let mut captured: Env = read_json(&layout::captured_file(project))?;
    captured.extend(values.iter().map(|(k, v)| (k.clone(), v.clone())));
    write_captured(project, &captured)
}

/// Replace everything in `.cactus/captured.json`.
pub(crate) fn replace_captured(project: &Path, values: &Env) -> Result<()> {
    let _guard = write_lock();
    write_captured(project, values)
}

fn write_captured(project: &Path, values: &Env) -> Result<()> {
    layout::ensure_workspace(project)?;
    write_json(&layout::captured_file(project), values)
}

/// A critter's captured values with where and when each came from.
pub fn load_context(project: &Path, critter: &str) -> Result<Context> {
    validate_critter_id(critter)?;
    read_json(&layout::critter_context_file(project, critter))
}

/// Save values `from` a request captured to the critter's context.
pub fn merge_context(project: &Path, critter: &str, values: &Env, from: &ProjectPath) -> Result<()> {
    validate_critter_id(critter)?;
    let at = now_unix_millis();
    update_context(project, critter, |context| {
        for (name, value) in values {
            context.insert(name.clone(), ContextEntry { value: value.clone(), from: from.to_string(), at });
        }
    })
}

/// Put back entries [`clear_context`] removed (Undo).
pub fn restore_context(project: &Path, critter: &str, entries: &Context) -> Result<()> {
    validate_critter_id(critter)?;
    update_context(project, critter, |context| context.extend(entries.clone()))
}

/// Remove the named entries, or all of them, and return what was removed.
pub fn clear_context(project: &Path, critter: &str, names: Option<&[String]>) -> Result<Context> {
    validate_critter_id(critter)?;
    let mut removed = Context::new();
    update_context(project, critter, |context| match names {
        None => removed = std::mem::take(context),
        Some(names) => {
            for name in names {
                if let Some(entry) = context.remove(name) {
                    removed.insert(name.clone(), entry);
                }
            }
        }
    })?;
    Ok(removed)
}

/// Make the context's values match `values`. Unchanged entries keep where
/// they came from; changed and new ones are marked [`EDITED`].
pub(crate) fn replace_context_values(project: &Path, critter: &str, values: &Env) -> Result<()> {
    validate_critter_id(critter)?;
    let at = now_unix_millis();
    update_context(project, critter, |context| {
        context.retain(|name, _| values.contains_key(name));
        for (name, value) in values {
            if context.get(name).is_none_or(|entry| entry.value != *value) {
                context.insert(name.clone(), ContextEntry { value: value.clone(), from: EDITED.into(), at });
            }
        }
    })
}

fn update_context(project: &Path, critter: &str, change: impl FnOnce(&mut Context)) -> Result<()> {
    let _guard = write_lock();
    let file = layout::critter_context_file(project, critter);
    let mut context: Context = read_json(&file)?;
    let before = context.clone();
    change(&mut context);
    if context == before {
        return Ok(());
    }
    ensure_critter_workspace(project, critter)?;
    write_json(&file, &context)
}

/// Create `.cactus/critters/<id>/`.
pub(crate) fn ensure_critter_workspace(project: &Path, critter: &str) -> Result<()> {
    layout::ensure_workspace(project)?;
    let dir = layout::critter_workspace_dir(project, critter);
    fs::create_dir_all(&dir).map_err(Error::file(&dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Env {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    #[test]
    fn context_records_sources_and_undoes_clears() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let login = ProjectPath::parse("auth/login.yml").unwrap();
        merge_context(p, "bob", &env(&[("token", "T"), ("id", "1")]), &login).unwrap();
        let context = load_context(p, "bob").unwrap();
        assert_eq!((context["token"].value.as_str(), context["token"].from.as_str()), ("T", "auth/login.yml"));
        assert!(context["token"].at > 0);

        let removed = clear_context(p, "bob", Some(&["token".into(), "nope".into()])).unwrap();
        assert_eq!(removed.keys().collect::<Vec<_>>(), ["token"]);
        assert_eq!(load_context(p, "bob").unwrap().len(), 1);
        restore_context(p, "bob", &removed).unwrap();
        assert_eq!(load_context(p, "bob").unwrap(), context);

        replace_context_values(p, "bob", &env(&[("token", "T"), ("new", "N")])).unwrap();
        let edited = load_context(p, "bob").unwrap();
        assert_eq!(edited.keys().collect::<Vec<_>>(), ["new", "token"]);
        assert_eq!((edited["token"].from.as_str(), edited["new"].from.as_str()), ("auth/login.yml", EDITED));

        assert_eq!(clear_context(p, "bob", None).unwrap().len(), 2);
        assert!(load_context(p, "bob").unwrap().is_empty());
        assert!(merge_context(p, "../x", &Env::new(), &login).is_err());
    }

    #[test]
    fn concurrent_captures_keep_every_value() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let from = ProjectPath::parse("a.yml").unwrap();
        std::thread::scope(|s| {
            for n in 0..16 {
                let (from, name) = (&from, format!("v{n}"));
                s.spawn(move || {
                    merge_context(p, "bob", &env(&[(&name, "x")]), from).unwrap();
                    merge_captured(p, &env(&[(&name, "x")])).unwrap();
                });
            }
        });
        assert_eq!(load_context(p, "bob").unwrap().len(), 16);
        assert_eq!(read_json::<Env>(&layout::captured_file(p)).unwrap().len(), 16);
    }
}
