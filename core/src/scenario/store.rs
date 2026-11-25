//! Reading, writing and organising request files.

use std::fs;
use std::path::Path;

use super::listing::{is_ignored, next_seq, walk};
use super::model::Scenario;
use crate::collection::model::LenientString;
use crate::text::slugify;
use crate::{Error, ProjectPath, Result, best_effort, collection, critter, history, layout};

const UNTITLED: &str = "untitled";
const COPY_SUFFIX: &str = " copy";

pub fn load(project: &Path, path: &ProjectPath) -> Result<Scenario> {
    collection::read(&path.resolve(project))
}

/// Save by editing the file in place: comments and anything cactus doesn't
/// edit stay as they are.
pub fn save(project: &Path, path: &ProjectPath, scenario: &Scenario) -> Result<()> {
    let mut scenario = scenario.clone();
    scenario.sync_query_params();
    collection::write(&path.resolve(project), &scenario)
}

/// Write `scenario` under `dir` with a free file name derived from its title,
/// placed after its siblings. Creates `opencollection.yml` and the folders'
/// `folder.yml` files if they're missing.
pub fn create(project: &Path, dir: &str, scenario: &Scenario) -> Result<ProjectPath> {
    collection::ensure_root(project)?;
    collection::ensure_folder(project, dir)?;
    let path = next_free_path(project, dir, &slug_for(&scenario.info.name), None)?;
    let mut scenario = scenario.clone();
    scenario.info.seq = Some(next_seq(project, dir));
    scenario.sync_query_params();
    collection::write(&path.resolve(project), &scenario)?;
    Ok(path)
}

pub fn create_blank(project: &Path, dir: &str, title: &str) -> Result<ProjectPath> {
    create(project, dir, &Scenario::titled(title))
}

/// A new request titled `title` that sends `method` to `url`.
pub fn create_request(project: &Path, dir: &str, title: &str, method: &str, url: &str) -> Result<ProjectPath> {
    let mut scenario = Scenario::titled(title);
    if !method.trim().is_empty() {
        scenario.http.method = LenientString(method.trim().to_uppercase());
    }
    scenario.http.url = LenientString(url.trim().to_string());
    create(project, dir, &scenario)
}

/// Give a request a new title, editing the file in place, then move it to a
/// file named after the title in the same folder, if it isn't already. Returns
/// where the request ends up.
pub fn retitle(project: &Path, path: &ProjectPath, title: &str) -> Result<ProjectPath> {
    let mut scenario = load(project, path)?;
    scenario.info.name = LenientString(title.trim().to_string());
    save(project, path, &scenario)?;
    let target = next_free_path(project, path.dir(), &slug_for(title), Some(path))?;
    if target != *path {
        rename(project, path, &target)?;
    }
    Ok(target)
}

/// Make folder `dir` and its `folder.yml`, for requests to be created in or
/// moved into. Refuses folders the listing wouldn't show as request folders.
pub fn create_folder(project: &Path, dir: &str) -> Result<()> {
    let path = ProjectPath::parse(dir.trim().trim_matches('/'))?;
    if !is_request_folder(path.as_str()) || is_ignored(path.as_str()) {
        return Err(Error::InvalidPath(dir.to_string()));
    }
    if layout::folder_file(project, path.as_str()).exists() {
        return Err(Error::AlreadyExists(path.to_string()));
    }
    collection::ensure_root(project)?;
    collection::ensure_folder(project, path.as_str())
}

/// Request folders with a `folder.yml`, empty ones included, sorted.
pub fn list_folders(project: &Path) -> Vec<String> {
    let mut folders: Vec<String> = walk(project)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == layout::FOLDER_FILE)
        .filter_map(|entry| ProjectPath::from_absolute(project, entry.path().parent()?))
        .map(|dir| dir.as_str().to_string())
        .filter(|dir| is_request_folder(dir))
        .collect();
    folders.sort();
    folders
}

/// Whether requests may live in `dir`: not the environments or critters folders.
fn is_request_folder(dir: &str) -> bool {
    let top = dir.split('/').next().unwrap_or(dir);
    top != layout::ENVIRONMENTS_DIR && top != layout::CRITTERS_DIR
}

/// Copy a request next to the original, titled "<title> copy". The copy is
/// written fresh, so the original's comments don't come along.
pub fn duplicate(project: &Path, path: &ProjectPath) -> Result<ProjectPath> {
    let mut scenario = load(project, path)?;
    let title = if scenario.info.name.is_empty() { UNTITLED } else { &scenario.info.name };
    scenario.info.name = format!("{title}{COPY_SUFFIX}").into();
    create(project, path.dir(), &scenario)
}

/// Delete a request along with its history.
pub fn delete(project: &Path, path: &ProjectPath) -> Result<()> {
    let file = path.resolve(project);
    fs::remove_file(&file).map_err(Error::file(&file))?;
    history::clear(project, path);
    Ok(())
}

/// Move a request (and its history), refusing to overwrite or to take a
/// name that would no longer be read as a request.
pub fn rename(project: &Path, from: &ProjectPath, to: &ProjectPath) -> Result<()> {
    if !layout::is_request_file(to.as_str()) {
        return Err(Error::InvalidPath(to.to_string()));
    }
    let source = from.resolve(project);
    let target = to.resolve(project);
    if target.exists() && !is_same_file(&source, &target) {
        return Err(Error::AlreadyExists(to.to_string()));
    }
    collection::ensure_folder(project, to.dir())?;
    fs::rename(&source, &target).map_err(Error::file(&source))?;
    history::rename(project, from, to);
    // The request has moved either way; a flow left pointing at the old
    // path shows that as a problem rather than failing the move.
    best_effort::log(
        "point flows at the moved request",
        critter::follow_moved_request(project, from.as_str(), to.as_str()),
    );
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
/// `current`, the file being renamed, counts as free.
fn next_free_path(project: &Path, dir: &str, slug: &str, current: Option<&ProjectPath>) -> Result<ProjectPath> {
    let extension = layout::REQUEST_EXTENSION;
    let candidates =
        std::iter::once(format!("{slug}{extension}")).chain((2..).map(|n| format!("{slug}-{n}{extension}")));
    for file_name in candidates {
        let candidate = ProjectPath::in_dir(dir, &file_name)?;
        if Some(&candidate) == current || !is_taken(project, &candidate) {
            return Ok(candidate);
        }
    }
    Err(Error::AlreadyExists(format!("{slug}{extension}")))
}

/// Names that would be read as collection, folder or environment files are
/// never free.
fn is_taken(project: &Path, candidate: &ProjectPath) -> bool {
    !layout::is_request_file(candidate.as_str()) || candidate.resolve(project).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::{Auth, AuthMode, Header};
    use crate::scenario::{Body, TypedBody};

    fn path(rel: &str) -> ProjectPath {
        ProjectPath::parse(rel).unwrap()
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
    fn retitle_edits_the_title_in_place_and_follows_with_the_file_name() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        fs::create_dir_all(p.join("auth")).unwrap();
        fs::write(p.join("auth/a.yml"), HAND_WRITTEN).unwrap();
        let moved = retitle(p, &path("auth/a.yml"), " Log in ").unwrap();
        assert_eq!(moved.as_str(), "auth/log-in.yml");
        let text = fs::read_to_string(moved.resolve(p)).unwrap();
        assert!(text.contains("name: Log in # the title") && text.contains("# Always JSON."), "{text}");
        assert!(!p.join("auth/a.yml").exists());

        // Its own name doesn't count as taken; a sibling's does.
        assert_eq!(retitle(p, &moved, "log in").unwrap(), moved);
        create_blank(p, "auth", "Other").unwrap();
        assert_eq!(retitle(p, &moved, "Other").unwrap().as_str(), "auth/other-2.yml");
    }

    #[test]
    fn create_request_sets_method_and_url() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let created = create_request(p, "users", "Get user", "post", " https://x/u ").unwrap();
        assert_eq!(created.as_str(), "users/get-user.yml");
        let s = load(p, &created).unwrap();
        assert_eq!((s.http.method.as_str(), s.http.url.as_str()), ("POST", "https://x/u"));
    }

    #[test]
    fn folders_are_created_and_listed_empty_or_not() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        create_folder(p, "users/admin").unwrap();
        create_blank(p, "auth", "a").unwrap();
        fs::create_dir_all(p.join("environments")).unwrap();
        fs::write(p.join("environments/folder.yml"), "").unwrap();
        assert_eq!(list_folders(p), ["auth", "users", "users/admin"]);
        assert!(matches!(create_folder(p, "users"), Err(Error::AlreadyExists(_))));
        for refused in ["environments", "critters/x", "../up", ".hidden", "node_modules"] {
            assert!(create_folder(p, refused).is_err(), "{refused}");
        }
    }

    #[test]
    fn rename_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let a = create_blank(p, "", "a").unwrap();
        let b = create_blank(p, "", "b").unwrap();
        assert!(matches!(rename(p, &a, &b), Err(Error::AlreadyExists(_))));
        assert!(matches!(rename(p, &a, &path("x/folder.yml")), Err(Error::InvalidPath(_))));
        rename(p, &a, &path("moved/a.yml")).unwrap();
        assert!(p.join("moved/a.yml").exists() && p.join("moved/folder.yml").exists());
    }

    #[test]
    fn flows_follow_a_request_that_moves() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let a = create_blank(p, "", "a").unwrap();
        crate::critter::create(p, "bob", &crate::critter::Critter::default()).unwrap();
        let text = "steps:\n  - request: a.yml # first\n  - tool: poll\n    request: a.yml\n  - request: other.yml\n";
        crate::critter::save_flow_text(p, "bob", "f", text).unwrap();

        rename(p, &a, &path("Test/a.yml")).unwrap();
        let moved = crate::critter::load_flow_text(p, "bob", "f").unwrap();
        assert_eq!(
            moved,
            "steps:\n  - request: Test/a.yml # first\n  - tool: poll\n    request: Test/a.yml\n  - request: other.yml\n"
        );
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
