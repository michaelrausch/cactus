//! Files cactus reads and writes are OpenCollection v1.0.0, and saving keeps
//! hand-written files intact.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use cactus_core::collection::{Auth, AuthMode, Header, Variable};
use cactus_core::env::{self, EnvFile};
use cactus_core::scenario::{
    self, Action, Assertion, Body, MultipartField, PartKind, PartValue, SetVariable, TypedBody,
};
use cactus_core::{ScenarioPath, import};
use serde_json::{Value, json};

const SCHEMA: &str = include_str!("fixtures/opencollection-v1.0.0.json");

fn demo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/demo")
}

/// Check `file` against the schema definition its role calls for.
fn assert_valid(project: &Path, file: &Path) {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    let rel = file.strip_prefix(project).unwrap().to_string_lossy().replace('\\', "/");
    let definition = match rel.as_str() {
        "opencollection.yml" => None,
        _ if rel.starts_with("environments/") => Some("Environment"),
        _ if rel.ends_with("/folder.yml") => Some("Folder"),
        _ => Some("HttpRequest"),
    };
    let schema = match definition {
        None => schema,
        Some(name) => {
            json!({"$schema": schema["$schema"], "$defs": schema["$defs"], "$ref": format!("#/$defs/{name}")})
        }
    };
    let instance: Value = serde_yaml_ng::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> =
        validator.iter_errors(&instance).map(|e| format!("{e} at {}", e.instance_path())).collect();
    assert!(errors.is_empty(), "{rel} is not valid OpenCollection:\n{}", errors.join("\n"));
}

fn yaml_files(project: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> =
        walk(project).into_iter().filter(|p| p.extension().is_some_and(|e| e == "yml")).collect();
    files.sort();
    files
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            let hidden = path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'));
            match () {
                () if hidden => vec![],
                () if path.is_dir() => walk(&path),
                () => vec![path],
            }
        })
        .collect()
}

#[test]
fn demo_is_valid_opencollection() {
    let files = yaml_files(&demo());
    assert_eq!(files.len(), 6);
    for file in files {
        assert_valid(&demo(), &file);
    }
}

#[test]
fn saving_the_demo_unchanged_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    for file in yaml_files(&demo()) {
        let target = dir.path().join(file.strip_prefix(demo()).unwrap());
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(&file, &target).unwrap();
    }
    let entries = scenario::list(dir.path()).unwrap();
    let order: Vec<_> = entries.iter().map(|e| e.rel_path.as_str()).collect();
    assert_eq!(order, ["auth/login.yml", "form-post.yml", "get-anything.yml"]);
    for entry in &entries {
        let before = fs::read_to_string(entry.rel_path.resolve(dir.path())).unwrap();
        let request = scenario::load(dir.path(), &entry.rel_path).unwrap();
        scenario::save(dir.path(), &entry.rel_path, &request).unwrap();
        assert_eq!(fs::read_to_string(entry.rel_path.resolve(dir.path())).unwrap(), before, "{}", entry.rel_path);
    }
    for layer in [EnvFile::Base, EnvFile::Named("staging".into())] {
        let file = if layer == EnvFile::Base { "opencollection.yml" } else { "environments/staging.yml" };
        let before = fs::read_to_string(dir.path().join(file)).unwrap();
        env::save(dir.path(), &layer, &env::load(dir.path(), &layer).unwrap()).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join(file)).unwrap(), before, "{file}");
    }
}

#[test]
fn everything_cactus_writes_is_valid() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();

    let blank = scenario::create_blank(p, "users/admin", "List users").unwrap();
    let mut request = scenario::load(p, &blank).unwrap();
    request.http.method = "POST".into();
    request.http.url = "{{baseUrl}}/users?page=1".into();
    request.http.headers =
        vec![Header::new("Accept", "application/json"), Header { disabled: true, ..Header::new("X-Off", "1") }];
    request.http.auth =
        Some(Auth::Mode(AuthMode::Basic { username: "{{user}}".into(), password: "{{password}}".into() }));
    request.http.body = Some(Body::Typed(TypedBody::MultipartForm {
        data: vec![MultipartField {
            name: "file".into(),
            kind: PartKind::File,
            value: PartValue::One("a.png".into()),
            disabled: false,
        }],
    }));
    request.runtime.variables = vec![Variable::new("page", "1")];
    request.runtime.assertions = vec![Assertion {
        expression: "res.status".into(),
        operator: "eq".into(),
        value: Some("201".into()),
        disabled: false,
    }];
    request.runtime.actions = vec![Action::SetVariable(SetVariable::new("id", "res.body.id"))];
    request.docs = "Creates a user.\nSecond line.".into();
    scenario::save(p, &blank, &request).unwrap();
    scenario::duplicate(p, &blank).unwrap();

    env::save(p, &EnvFile::Base, &BTreeMap::from([("baseUrl".into(), "http://localhost".into())])).unwrap();
    env::save(p, &EnvFile::Named("dev".into()), &BTreeMap::from([("user".into(), "dev".into())])).unwrap();

    let openapi = p.join("api.json");
    fs::write(
        &openapi,
        json!({"openapi": "3.0.0", "paths": {"/pets/{id}": {"get": {"summary": "Get pet", "tags": ["Pets"]}}}})
            .to_string(),
    )
    .unwrap();
    import::import(p, &openapi).unwrap();

    let files = yaml_files(p);
    let rels: Vec<String> =
        files.iter().map(|f| f.strip_prefix(p).unwrap().to_string_lossy().replace('\\', "/")).collect();
    assert_eq!(
        rels,
        [
            "environments/dev.yml",
            "opencollection.yml",
            "pets/folder.yml",
            "pets/get-pet.yml",
            "users/admin/folder.yml",
            "users/admin/list-users-copy.yml",
            "users/admin/list-users.yml",
            "users/folder.yml",
        ]
    );
    for file in &files {
        assert_valid(p, file);
    }
    let saved = scenario::load(p, &ScenarioPath::parse("users/admin/list-users.yml").unwrap()).unwrap();
    assert_eq!(saved, {
        request.sync_query_params();
        request
    });
}

#[test]
#[should_panic(expected = "is not valid OpenCollection")]
fn schema_check_rejects_unknown_keys() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bad.yml");
    fs::write(&file, "info:\n  name: Bad\n  type: http\nhttp:\n  method: GET\n  bogus: 1\n").unwrap();
    assert_valid(dir.path(), &file);
}
