//! Import Postman v2 collections and OpenAPI 3 / Swagger 2 specs as
//! OpenCollection request files.

mod openapi;
mod postman;

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::env::{self, EnvFile};
use crate::scenario::{self, Scenario};
use crate::text::slugify;
use crate::{Error, Result, ScenarioPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ImportFormat {
    Postman,
    OpenApi,
}

impl ImportFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Postman => "postman",
            Self::OpenApi => "openapi",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ImportSummary {
    pub format: ImportFormat,
    pub created: Vec<ScenarioPath>,
    /// Variables added to the collection; existing ones are never overwritten.
    pub env_vars: Vec<String>,
}

/// A request plus the folder it belongs in.
struct ImportedScenario {
    dir: String,
    scenario: Scenario,
}

/// What a format-specific parser extracts from a document.
#[derive(Default)]
struct ParsedCollection {
    scenarios: Vec<ImportedScenario>,
    variables: BTreeMap<String, String>,
}

pub fn import(project: &Path, file: &Path) -> Result<ImportSummary> {
    let document = parse_document(file)?;
    let format = detect_format(&document)?;
    let parsed = match format {
        ImportFormat::Postman => postman::parse(&document),
        ImportFormat::OpenApi => openapi::parse(&document),
    };
    let created = write_scenarios(project, &parsed.scenarios)?;
    let env_vars = add_missing_env_vars(project, parsed.variables)?;
    Ok(ImportSummary { format, created, env_vars })
}

/// YAML by extension; otherwise JSON, falling back to YAML.
fn parse_document(file: &Path) -> Result<Value> {
    let text = fs::read_to_string(file)?;
    let is_yaml =
        file.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"));
    if is_yaml {
        return Ok(serde_yaml_ng::from_str(&text)?);
    }
    Ok(serde_json::from_str(&text).or_else(|_| serde_yaml_ng::from_str(&text))?)
}

fn detect_format(document: &Value) -> Result<ImportFormat> {
    let is_postman = document["info"]["schema"].as_str().is_some_and(|schema| schema.contains("postman"));
    let is_openapi = document.get("openapi").is_some() || document.get("swagger").is_some();
    match (is_postman, is_openapi) {
        (true, _) => Ok(ImportFormat::Postman),
        (false, true) => Ok(ImportFormat::OpenApi),
        (false, false) => Err(Error::UnrecognizedImportFormat),
    }
}

fn write_scenarios(project: &Path, scenarios: &[ImportedScenario]) -> Result<Vec<ScenarioPath>> {
    scenarios.iter().map(|imported| scenario::create(project, &imported.dir, &imported.scenario)).collect()
}

/// Add variables the collection doesn't define yet; returns their names.
fn add_missing_env_vars(project: &Path, variables: BTreeMap<String, String>) -> Result<Vec<String>> {
    let mut base = env::load(project, &EnvFile::Base)?;
    let mut added = Vec::new();
    for (name, value) in variables {
        if let Entry::Vacant(slot) = base.entry(name.clone()) {
            slot.insert(value);
            added.push(name);
        }
    }
    if !added.is_empty() {
        env::save(project, &EnvFile::Base, &base)?;
    }
    Ok(added)
}

/// The folder for a group called `name` inside folder `parent`.
fn child_dir(parent: &str, name: &str) -> String {
    let slug = slugify(name);
    match (parent.is_empty(), slug.is_empty()) {
        (_, true) => parent.to_string(),
        (true, false) => slug,
        (false, false) => format!("{parent}/{slug}"),
    }
}

/// Descriptive text that may be a plain string or `{"content": "..."}`.
fn description_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Object(o) => o.get("content").and_then(Value::as_str).unwrap_or_default().to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::collection::{Auth, AuthMode};
    use crate::scenario::{Body, ParamKind, PartKind, PartValue, TypedBody};

    fn path(rel: &str) -> ScenarioPath {
        ScenarioPath::parse(rel).unwrap()
    }

    #[test]
    fn imports_postman_collection() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        env::save(p, &EnvFile::Base, &BTreeMap::from([("baseUrl".into(), "keep-me".into())])).unwrap();
        let file = p.join("c.json");
        fs::write(
            &file,
            json!({
                "info": {"name": "C", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
                "variable": [{"key": "baseUrl", "value": "x"}, {"key": "token", "value": "t"}],
                "item": [
                    {"name": "Users", "item": [{
                        "name": "Create user",
                        "event": [{"listen": "test", "script": {"exec": ["pm.test('ok');"]}}],
                        "request": {
                            "method": "post",
                            "url": {"raw": "{{baseUrl}}/users"},
                            "header": [{"key": "A", "value": "1"}, {"key": "B", "value": "2", "disabled": true}],
                            "body": {"mode": "raw", "raw": "{\"n\":1}", "options": {"raw": {"language": "json"}}},
                            "auth": {"type": "bearer", "bearer": [{"key": "token", "value": "{{token}}"}]}
                        }
                    }]},
                    {"name": "Upload", "request": {"method": "POST", "url": "{{baseUrl}}/up",
                        "body": {"mode": "formdata", "formdata": [{"key": "f", "type": "file", "src": "a.png"}, {"key": "t", "value": "x"}]}}},
                    {"name": "Ping", "request": "{{baseUrl}}/ping"}
                ]
            })
            .to_string(),
        )
        .unwrap();
        let sum = import(p, &file).unwrap();
        let created: Vec<_> = sum.created.iter().map(ScenarioPath::as_str).collect();
        assert_eq!(created, ["users/create-user.yml", "upload.yml", "ping.yml"]);
        assert_eq!(sum.env_vars, ["token"]);
        assert_eq!(env::load(p, &EnvFile::Base).unwrap()["baseUrl"], "keep-me");

        let s = scenario::load(p, &path("users/create-user.yml")).unwrap();
        assert_eq!((s.http.method.as_str(), s.http.url.as_str()), ("POST", "{{baseUrl}}/users"));
        assert_eq!(s.http.body, Some(Body::Typed(TypedBody::Json { data: r#"{"n":1}"#.into() })));
        assert_eq!((s.http.headers[0].name.as_str(), s.http.headers[1].disabled), ("A", true));
        assert_eq!(s.http.auth, Some(Auth::Mode(AuthMode::Bearer { token: "{{token}}".into() })));
        assert_eq!(s.runtime.scripts[0].code, "pm.test('ok');");
        let up = scenario::load(p, &path("upload.yml")).unwrap();
        let Some(Body::Typed(TypedBody::MultipartForm { data })) = up.http.body else { panic!("multipart") };
        assert_eq!((data[0].kind, &data[0].value), (PartKind::File, &PartValue::One("a.png".into())));
        assert_eq!(up.http.auth, Some(Auth::INHERIT));
    }

    #[test]
    fn imports_openapi_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let file = p.join("api.yaml");
        fs::write(
            &file,
            r#"
openapi: 3.0.0
info: {title: Pets, version: "1"}
servers: [{url: "https://api.example.com/v1"}]
paths:
  /pets/{petId}:
    get:
      summary: Get a pet
      tags: [Pets]
  /pets:
    post:
      operationId: createPet
      tags: [Pets]
      requestBody:
        content:
          application/json:
            example: {name: Rex}
"#,
        )
        .unwrap();
        let sum = import(p, &file).unwrap();
        assert_eq!(sum.format, ImportFormat::OpenApi);
        assert_eq!(sum.created.len(), 2);
        assert_eq!(env::load(p, &EnvFile::Base).unwrap()["baseUrl"], "https://api.example.com/v1");
        let get = scenario::load(p, &path("pets/get-a-pet.yml")).unwrap();
        assert_eq!(get.http.url, "{{baseUrl}}/pets/:petId");
        assert_eq!((get.http.params[0].name.as_str(), get.http.params[0].kind), ("petId", ParamKind::Path));
        let post = scenario::load(p, &path("pets/createpet.yml")).unwrap();
        assert_eq!(post.http.body, Some(Body::Typed(TypedBody::Json { data: "{\n  \"name\": \"Rex\"\n}".into() })));
    }

    #[test]
    fn rejects_unknown_format() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("x.json");
        fs::write(&file, "{}").unwrap();
        assert!(matches!(import(dir.path(), &file), Err(Error::UnrecognizedImportFormat)));
    }

    #[test]
    fn child_dirs() {
        assert_eq!(child_dir("", "My Folder"), "my-folder");
        assert_eq!(child_dir("a", "B"), "a/b");
        assert_eq!(child_dir("a", "!!"), "a");
    }
}
