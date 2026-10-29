//! Reading, writing and organising scenario files.

use std::fs;
use std::path::Path;

use super::model::Scenario;
use crate::storage::{ensure_parent_dir, write_json};
use crate::text::slugify;
use crate::{Error, Result, ScenarioPath, history, layout};

const UNTITLED: &str = "untitled";
const COPY_SUFFIX: &str = " copy";
const SCENARIO_EXTENSION: &str = ".json";

pub fn load(project: &Path, path: &ScenarioPath) -> Result<Scenario> {
    Ok(serde_json::from_slice(&fs::read(path.resolve(project))?)?)
}

pub fn save(project: &Path, path: &ScenarioPath, scenario: &Scenario) -> Result<()> {
    write_json(&path.resolve(project), scenario)
}

/// Write `scenario` under `dir` with a free file name derived from its title.
pub fn create(project: &Path, dir: &str, scenario: &Scenario) -> Result<ScenarioPath> {
    let path = next_free_path(project, dir, &slug_for(&scenario.title))?;
    let file = path.resolve(project);
    ensure_parent_dir(&file)?;
    write_json(&file, scenario)?;
    Ok(path)
}

pub fn create_blank(project: &Path, dir: &str, title: &str) -> Result<ScenarioPath> {
    create(project, dir, &Scenario::titled(title))
}

/// Copy a scenario next to the original, titled "<title> copy".
pub fn duplicate(project: &Path, path: &ScenarioPath) -> Result<ScenarioPath> {
    let mut scenario = load(project, path)?;
    let title = if scenario.title.is_empty() { UNTITLED } else { &scenario.title };
    scenario.title = format!("{title}{COPY_SUFFIX}");
    create(project, path.dir(), &scenario)
}

/// Delete a scenario along with its history.
pub fn delete(project: &Path, path: &ScenarioPath) -> Result<()> {
    fs::remove_file(path.resolve(project))?;
    history::clear(project, path);
    Ok(())
}

/// Move a scenario (and its history), refusing to overwrite.
pub fn rename(project: &Path, from: &ScenarioPath, to: &ScenarioPath) -> Result<()> {
    let target = to.resolve(project);
    if target.exists() {
        return Err(Error::AlreadyExists(to.to_string()));
    }
    ensure_parent_dir(&target)?;
    fs::rename(from.resolve(project), target)?;
    history::rename(project, from, to);
    Ok(())
}

fn slug_for(title: &str) -> String {
    let slug = slugify(title);
    if slug.is_empty() { UNTITLED.to_string() } else { slug }
}

/// `slug.json`, else `slug-2.json`, `slug-3.json`, … whichever is free first.
fn next_free_path(project: &Path, dir: &str, slug: &str) -> Result<ScenarioPath> {
    for n in 1.. {
        let file_name =
            if n == 1 { format!("{slug}{SCENARIO_EXTENSION}") } else { format!("{slug}-{n}{SCENARIO_EXTENSION}") };
        let candidate = ScenarioPath::in_dir(dir, &file_name)?;
        if !is_taken(project, &candidate) {
            return Ok(candidate);
        }
    }
    unreachable!("an unbounded range always yields a free name")
}

/// Root-level env file names are reserved: they would be read as variables.
fn is_taken(project: &Path, candidate: &ScenarioPath) -> bool {
    let reserved = candidate.dir().is_empty() && layout::is_env_file(candidate.file_name());
    reserved || candidate.resolve(project).exists()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::scenario::Auth;

    fn path(rel: &str) -> ScenarioPath {
        ScenarioPath::parse(rel).unwrap()
    }

    #[test]
    fn round_trip_preserves_unknown_keys() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("a.json"),
            r#"{"title":"A","url":"http://x","custom":{"k":1},"body":{"x":1},"auth":{"type":"bearer","token":"t"},"expect":{"status":[200,201]}}"#,
        )
        .unwrap();
        let mut s = load(dir.path(), &path("a.json")).unwrap();
        assert_eq!(s.method, "GET");
        assert_eq!(s.auth, Some(Auth::Bearer { token: "t".into() }));
        assert!(s.expect.as_ref().unwrap().status.as_ref().unwrap().accepts(201));
        s.title = "B".into();
        save(dir.path(), &path("a.json"), &s).unwrap();
        let raw: Value = serde_json::from_slice(&fs::read(dir.path().join("a.json")).unwrap()).unwrap();
        assert_eq!(raw["custom"]["k"], 1);
        assert_eq!(raw["title"], "B");
        assert_eq!(raw["body"]["x"], 1);
        // Empty optional sections stay out of the file.
        assert!(raw.get("capture").is_none() && raw.get("disabledHeaders").is_none());
    }

    #[test]
    fn create_and_duplicate_pick_free_names() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let created = |dir: &str, title: &str| create_blank(p, dir, title).unwrap().to_string();
        assert_eq!(created("", "Get User!"), "get-user.json");
        assert_eq!(created("", "Get User!"), "get-user-2.json");
        assert_eq!(created("auth", ""), "auth/untitled.json");
        assert_eq!(created("", "env"), "env-2.json");
        assert_eq!(duplicate(p, &path("auth/untitled.json")).unwrap().as_str(), "auth/untitled-copy.json");
        assert_eq!(load(p, &path("auth/untitled-copy.json")).unwrap().title, "untitled copy");
    }

    #[test]
    fn rename_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let a = create_blank(p, "", "a").unwrap();
        let b = create_blank(p, "", "b").unwrap();
        assert!(matches!(rename(p, &a, &b), Err(Error::AlreadyExists(_))));
        rename(p, &a, &path("moved/a.json")).unwrap();
        assert!(p.join("moved/a.json").exists());
    }
}
