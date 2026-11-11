//! The collection around the requests: `opencollection.yml` at the root,
//! a `folder.yml` per folder and `environments/*.yml`, all OpenCollection
//! v1.0.0 in the layout Bruno uses.

mod emit;
mod file;
pub mod model;
mod patch;

use std::fs;
use std::path::Path;

pub(crate) use emit::to_yaml;
pub use file::{read, read_if_exists, write};
pub use model::{
    ApiKeyPlacement, Auth, AuthMode, CollectionRoot, Environment, Folder, Header, Inherit, LenientString,
    RequestDefaults, Variable, VariableValue,
};

use crate::{Error, Result, layout, scenario};

/// The project's `opencollection.yml`, or a fresh one named after the folder.
pub fn load_root(project: &Path) -> Result<CollectionRoot> {
    Ok(read_if_exists(&layout::collection_file(project))?
        .unwrap_or_else(|| CollectionRoot::named(folder_name(project))))
}

pub fn save_root(project: &Path, root: &CollectionRoot) -> Result<()> {
    write(&layout::collection_file(project), root)
}

/// Write `opencollection.yml` if the project has none, so Bruno and cactus
/// recognise the folder as a collection.
pub fn ensure_root(project: &Path) -> Result<()> {
    let file = layout::collection_file(project);
    if !file.exists() {
        write(&file, &CollectionRoot::named(folder_name(project)))?;
    }
    Ok(())
}

/// Create folder `dir` (`""` is the root) and a `folder.yml` for it and
/// every parent that lacks one.
pub fn ensure_folder(project: &Path, dir: &str) -> Result<()> {
    let mut parent = String::new();
    for (path, name) in layout::folder_prefixes(dir) {
        let folder = project.join(&path);
        fs::create_dir_all(&folder).map_err(Error::file(&folder))?;
        let file = layout::folder_file(project, &path);
        if !file.exists() {
            write(&file, &Folder::named(name, scenario::next_seq(project, &parent)))?;
        }
        parent = path;
    }
    Ok(())
}

/// What a request in `dir` inherits, outermost first: the collection's
/// defaults, then each folder's down to `dir`. Folders without a
/// `folder.yml` contribute nothing.
pub fn defaults_chain(project: &Path, dir: &str) -> Result<Vec<RequestDefaults>> {
    let mut chain = vec![load_root(project)?.request];
    chain.extend(folder_defaults(project, dir)?.into_iter().map(|(_, defaults)| defaults));
    Ok(chain)
}

/// The folders' part of [`defaults_chain`], each with its path.
pub fn folder_defaults(project: &Path, dir: &str) -> Result<Vec<(String, RequestDefaults)>> {
    let mut folders = Vec::new();
    for (path, _) in layout::folder_prefixes(dir) {
        if let Some(folder) = read_if_exists::<Folder>(&layout::folder_file(project, &path))? {
            folders.push((path, folder.request));
        }
    }
    Ok(folders)
}

fn folder_name(project: &Path) -> String {
    project.file_name().map_or_else(|| "collection".into(), |name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        ensure_root(p).unwrap();
        assert_eq!(read::<CollectionRoot>(&p.join("opencollection.yml")).unwrap().opencollection, "1.0.0");
        ensure_folder(p, "users/admin").unwrap();
        ensure_folder(p, "orders").unwrap();
        let users: Folder = read(&p.join("users/folder.yml")).unwrap();
        let orders: Folder = read(&p.join("orders/folder.yml")).unwrap();
        assert_eq!((users.info.name.as_str(), users.info.seq), ("users", Some(1.0)));
        assert_eq!(orders.info.seq, Some(2.0));
        assert!(p.join("users/admin/folder.yml").exists());

        let mut root = load_root(p).unwrap();
        root.request.headers.push(Header::new("X-Root", "1"));
        save_root(p, &root).unwrap();
        let chain = defaults_chain(p, "users/admin").unwrap();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].headers[0].name, "X-Root");
    }
}
