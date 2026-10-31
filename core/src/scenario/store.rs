//! Reading, writing and organising request files.

use std::fs;
use std::path::Path;

use super::listing::next_seq;
use super::model::Scenario;
use crate::text::slugify;
use crate::{Error, Result, ScenarioPath, collection, history, layout};

const UNTITLED: &str = "untitled";
const COPY_SUFFIX: &str = " copy";

pub fn load(project: &Path, path: &ScenarioPath) -> Result<Scenario> {
    collection::read(&path.resolve(project))
}

/// Save by editing the file in place: comments and anything cactus doesn't
/// edit stay as they are.
pub fn save(project: &Path, path: &ScenarioPath, scenario: &Scenario) -> Result<()> {
    let mut scenario = scenario.clone();
    scenario.sync_query_params();
    collection::write(&path.resolve(project), &scenario)
}

/// Write `scenario` under `dir` with a free file name derived from its title,
/// placed after its siblings. Creates `opencollection.yml` and the folders'
/// `folder.yml` files if they're missing.
pub fn create(project: &Path, dir: &str, scenario: &Scenario) -> Result<ScenarioPath> {
    collection::ensure_root(project)?;
    collection::ensure_folder(project, dir)?;
    let path = next_free_path(project, dir, &slug_for(&scenario.info.name))?;
    let mut scenario = scenario.clone();
    scenario.info.seq = Some(next_seq(project, dir));
    scenario.sync_query_params();
    collection::write(&path.resolve(project), &scenario)?;
    Ok(path)
}

pub fn create_blank(project: &Path, dir: &str, title: &str) -> Result<ScenarioPath> {
    create(project, dir, &Scenario::titled(title))
}

/// Copy a request next to the original, titled "<title> copy". The copy is
/// written fresh, so the original's comments don't come along.
pub fn duplicate(project: &Path, path: &ScenarioPath) -> Result<ScenarioPath> {
    let mut scenario = load(project, path)?;
    let title = if scenario.info.name.is_empty() { UNTITLED } else { &scenario.info.name };
    scenario.info.name = format!("{title}{COPY_SUFFIX}");
    create(project, path.dir(), &scenario)
}

/// Delete a request along with its history.
pub fn delete(project: &Path, path: &ScenarioPath) -> Result<()> {
    fs::remove_file(path.resolve(project))?;
    history::clear(project, path);
    Ok(())
}

/// Move a request (and its history), refusing to overwrite or to take a
/// name that would no longer be read as a request.
pub fn rename(project: &Path, from: &ScenarioPath, to: &ScenarioPath) -> Result<()> {
    if !layout::is_request_file(to.as_str()) {
        return Err(Error::InvalidScenarioPath(to.to_string()));
    }
    let source = from.resolve(project);
    let target = to.resolve(project);
    if target.exists() && !is_same_file(&source, &target) {
        return Err(Error::AlreadyExists(to.to_string()));
    }
    collection::ensure_folder(project, to.dir())?;
    fs::rename(source, target)?;
    history::rename(project, from, to);
    Ok(())
}

/// On a case-insensitive file system (Windows, macOS) `login.yml` and
/// `Login.yml` are one file, so renaming one to the other only changes case.
fn is_same_file(a: &Path, b: &Path) -> bool {
    matches!((fs::canonicalize(a), fs::canonicalize(b)), (Ok(a), Ok(b)) if a == b)
}

fn slug_for(title: &str) -> String {
    let slug = slugify(title);
    if slug.is_empty() { UNTITLED.to_string() } else { slug }
}

/// `slug.yml`, else `slug-2.yml`, `slug-3.yml`, … whichever is free first.
fn next_free_path(project: &Path, dir: &str, slug: &str) -> Result<ScenarioPath> {
    let extension = layout::REQUEST_EXTENSION;
    for n in 1.. {
        let file_name = if n == 1 { format!("{slug}{extension}") } else { format!("{slug}-{n}{extension}") };
        let candidate = ScenarioPath::in_dir(dir, &file_name)?;
        if !is_taken(project, &candidate) {
            return Ok(candidate);
        }
    }
    unreachable!("an unbounded range always yields a free name")
}

/// Names that would be read as collection, folder or environment files are
/// never free.
fn is_taken(project: &Path, candidate: &ScenarioPath) -> bool {
    !layout::is_request_file(candidate.as_str()) || candidate.resolve(project).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::{Auth, AuthMode, Header};
    use crate::scenario::{Body, TypedBody};

    fn path(rel: &str) -> ScenarioPath {
        ScenarioPath::parse(rel).unwrap()
    }

    const HAND_WRITTEN: &str = r#"# Hand-written; comments must survive.
info:
  name: A # the title
  type: http
  tags: [smoke]

http:
  method: GET
  url: "http://x/?a=1"
  headers:
    # Always JSON.
    - name: Accept
      value: application/json
      description: Kept although cactus doesn't model it.
  auth:
    type: bearer
    token: t

examples:
  - name: ok
"#;

    #[test]
    fn save_patches_in_place() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.yml"), HAND_WRITTEN).unwrap();
        let mut s = load(dir.path(), &path("a.yml")).unwrap();
        assert_eq!(s.http.auth, Some(Auth::Mode(AuthMode::Bearer { token: "t".into() })));
        save(dir.path(), &path("a.yml"), &s).unwrap();
        // Only the query param mirroring the URL was added.
        let text = fs::read_to_string(dir.path().join("a.yml")).unwrap();
        assert!(text.starts_with(&HAND_WRITTEN[..HAND_WRITTEN.find("  auth:").unwrap()]), "{text}");
        assert!(text.contains("  params:\n    - name: a\n      value: '1'\n      type: query\n"), "{text}");

        s = load(dir.path(), &path("a.yml")).unwrap();
        s.info.name = "B".into();
        s.http.headers[0].value = "text/plain".into();
        s.http.headers.push(Header::new("X-New", "1"));
        s.http.body = Some(Body::Typed(TypedBody::Json { data: "{\n  \"n\": 1\n}".into() }));
        save(dir.path(), &path("a.yml"), &s).unwrap();
        let text = fs::read_to_string(dir.path().join("a.yml")).unwrap();
        for kept in [
            "# Hand-written",
            "name: B # the title",
            "tags: [smoke]",
            "# Always JSON.",
            "description: Kept",
            "examples:",
        ] {
            assert!(text.contains(kept), "lost {kept:?}:\n{text}");
        }
        assert!(text.contains("value: text/plain"), "{text}");
        assert!(text.contains("  body:\n    type: json\n    data: |-\n      {\n        \"n\": 1\n      }\n"), "{text}");
        assert_eq!(load(dir.path(), &path("a.yml")).unwrap().http.headers[1].name, "X-New");
    }

    #[test]
    fn create_and_duplicate_pick_free_names_and_seq() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let created = |dir: &str, title: &str| create_blank(p, dir, title).unwrap().to_string();
        assert_eq!(created("", "Get User!"), "get-user.yml");
        assert_eq!(created("", "Get User!"), "get-user-2.yml");
        assert_eq!(created("auth", ""), "auth/untitled.yml");
        assert_eq!(created("", "opencollection"), "opencollection-2.yml");
        assert_eq!(created("auth", "folder"), "auth/folder-2.yml");
        assert_eq!(duplicate(p, &path("auth/untitled.yml")).unwrap().as_str(), "auth/untitled-copy.yml");
        let copy = load(p, &path("auth/untitled-copy.yml")).unwrap();
        assert_eq!((copy.info.name.as_str(), copy.info.seq), ("untitled copy", Some(3.0)));
        assert_eq!(copy.http.auth, Some(Auth::INHERIT));
        assert!(p.join("opencollection.yml").exists() && p.join("auth/folder.yml").exists());
        assert_eq!(load(p, &path("get-user-2.yml")).unwrap().info.seq, Some(2.0));
    }

    #[test]
    fn rename_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let a = create_blank(p, "", "a").unwrap();
        let b = create_blank(p, "", "b").unwrap();
        assert!(matches!(rename(p, &a, &b), Err(Error::AlreadyExists(_))));
        assert!(matches!(rename(p, &a, &path("x/folder.yml")), Err(Error::InvalidScenarioPath(_))));
        rename(p, &a, &path("moved/a.yml")).unwrap();
        assert!(p.join("moved/a.yml").exists() && p.join("moved/folder.yml").exists());
    }

    #[test]
    fn rename_changes_case_only_where_names_are_one_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let lower = create_blank(p, "", "login").unwrap();
        let upper = path("Login.yml");
        let case_insensitive = upper.resolve(p).exists();
        if case_insensitive {
            rename(p, &lower, &upper).unwrap();
            let names: Vec<_> = fs::read_dir(p).unwrap().map(|e| e.unwrap().file_name()).collect();
            assert!(names.iter().any(|n| n == "Login.yml"), "{names:?}");
        } else {
            fs::write(upper.resolve(p), "").unwrap();
            assert!(matches!(rename(p, &lower, &upper), Err(Error::AlreadyExists(_))));
        }
    }
}
