//! Turning a [`Scenario`] into an outgoing request.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, RequestBuilder, multipart};
use serde_json::{Map, Value};

use crate::env::{Env, Substituter};
use crate::scenario::{Auth, BodyType, Scenario};
use crate::text::plain_string;
use crate::{Error, Result};

/// A multipart value starting with this is a file path, relative to the project.
pub const ATTACHMENT_PREFIX: char = '@';
const JSON_CONTENT_TYPE: &str = "application/json";

pub struct PreparedRequest {
    pub request: reqwest::Request,
    /// `{{names}}` that no env layer defines, left as-is in the request.
    pub unresolved: Vec<String>,
}

/// Build the outgoing request with env vars applied. `project` anchors
/// relative attachment paths.
pub fn build_request(
    client: &reqwest::Client,
    project: &Path,
    scenario: &Scenario,
    env: &Env,
) -> Result<PreparedRequest> {
    let mut vars = Substituter::new(env);
    let url = vars.apply(scenario.url.trim());
    let method = parse_method(&scenario.method)?;
    // Auth goes first so an explicit Authorization header overrides it.
    let builder = apply_auth(client.request(method, &url), scenario.auth.as_ref(), &mut vars);
    let mut headers = build_headers(&scenario.headers, &mut vars)?;
    let builder = attach_body(builder, &mut headers, project, scenario, &mut vars)?;
    let request = builder.headers(headers).build()?;
    Ok(PreparedRequest { request, unresolved: vars.into_unresolved() })
}

fn parse_method(method: &str) -> Result<Method> {
    Method::from_bytes(method.trim().to_uppercase().as_bytes()).map_err(|_| Error::InvalidMethod(method.to_string()))
}

fn apply_auth(builder: RequestBuilder, auth: Option<&Auth>, vars: &mut Substituter) -> RequestBuilder {
    match auth {
        Some(Auth::Bearer { token }) => builder.bearer_auth(vars.apply(token)),
        Some(Auth::Basic { username, password }) => {
            builder.basic_auth(vars.apply(username), Some(vars.apply(password)))
        }
        None => builder,
    }
}

/// Rows with a blank name are ignored; the editor leaves them while typing.
fn build_headers(headers: &BTreeMap<String, String>, vars: &mut Substituter) -> Result<HeaderMap> {
    let mut map = HeaderMap::new();
    for (name, value) in headers.iter().filter(|(name, _)| !name.trim().is_empty()) {
        let name = vars.apply(name.trim());
        let value = vars.apply(value);
        let header_name =
            HeaderName::from_bytes(name.as_bytes()).map_err(|_| Error::InvalidHeaderName(name.clone()))?;
        let header_value = HeaderValue::from_str(&value).map_err(|_| Error::InvalidHeaderValue(name))?;
        map.insert(header_name, header_value);
    }
    Ok(map)
}

fn attach_body(
    builder: RequestBuilder,
    headers: &mut HeaderMap,
    project: &Path,
    scenario: &Scenario,
    vars: &mut Substituter,
) -> Result<RequestBuilder> {
    let body = &scenario.body;
    Ok(match scenario.effective_body_type() {
        BodyType::None => builder,
        BodyType::Text => text_body(builder, body, vars),
        BodyType::Json => {
            headers.entry(CONTENT_TYPE).or_insert(HeaderValue::from_static(JSON_CONTENT_TYPE));
            json_body(builder, body, vars)
        }
        BodyType::Form => form_body(builder, body, vars)?,
        BodyType::Multipart => multipart_body(builder, project, body, vars)?,
    })
}

fn text_body(builder: RequestBuilder, body: &Value, vars: &mut Substituter) -> RequestBuilder {
    if body.is_null() { builder } else { builder.body(vars.apply(&plain_string(body))) }
}

/// A string body is raw JSON text (possibly with placeholders outside
/// strings); anything else is serialised first.
fn json_body(builder: RequestBuilder, body: &Value, vars: &mut Substituter) -> RequestBuilder {
    match body {
        Value::Null => builder,
        Value::String(raw) => builder.body(vars.apply(raw)),
        other => builder.body(vars.apply(&other.to_string())),
    }
}

fn form_body(builder: RequestBuilder, body: &Value, vars: &mut Substituter) -> Result<RequestBuilder> {
    let fields: Vec<(String, String)> =
        body_fields(body, BodyType::Form)?.iter().map(|(k, v)| (vars.apply(k), vars.apply(&plain_string(v)))).collect();
    Ok(builder.form(&fields))
}

fn multipart_body(
    builder: RequestBuilder,
    project: &Path,
    body: &Value,
    vars: &mut Substituter,
) -> Result<RequestBuilder> {
    let mut form = multipart::Form::new();
    for (name, value) in body_fields(body, BodyType::Multipart)? {
        let name = vars.apply(name);
        let value = vars.apply(&plain_string(value));
        form = match parse_attachment(&value) {
            Some(file) => form.part(name, read_attachment(project, file)?),
            None => form.text(name, value),
        };
    }
    Ok(builder.multipart(form))
}

fn body_fields(body: &Value, body_type: BodyType) -> Result<&Map<String, Value>> {
    body.as_object().ok_or(Error::BodyNotAnObject(body_type.as_str()))
}

fn parse_attachment(value: &str) -> Option<&str> {
    value.strip_prefix(ATTACHMENT_PREFIX)
}

fn read_attachment(project: &Path, file: &str) -> Result<multipart::Part> {
    let path = project.join(file);
    let data = fs::read(&path).map_err(|source| Error::AttachmentUnreadable { path: path.clone(), source })?;
    let file_name = path.file_name().map_or_default(|n| n.to_string_lossy().into_owned());
    Ok(multipart::Part::bytes(data).file_name(file_name))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::http::build_client;

    fn body_of(req: &reqwest::Request) -> String {
        String::from_utf8(req.body().unwrap().as_bytes().unwrap().to_vec()).unwrap()
    }

    fn build(project: &Path, s: &Scenario, env: &Env) -> Result<(reqwest::Request, Vec<String>)> {
        build_request(&build_client(), project, s, env).map(|p| (p.request, p.unresolved))
    }

    #[test]
    fn build_applies_env_and_json_body() {
        let env = Env::from([("base".into(), "http://localhost:1".into()), ("tok".into(), "abc".into())]);
        let s = Scenario {
            method: "post".into(),
            url: "{{base}}/x".into(),
            headers: BTreeMap::from([("X-Tok".into(), "{{tok}}".into())]),
            disabled_headers: BTreeMap::from([("X-Off".into(), "1".into())]),
            body: json!({"a": "{{missing}}"}),
            ..Default::default()
        };
        let (req, un) = build(Path::new("."), &s, &env).unwrap();
        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.url().as_str(), "http://localhost:1/x");
        assert_eq!(req.headers()["x-tok"], "abc");
        assert!(req.headers().get("x-off").is_none());
        assert_eq!(req.headers()["content-type"], "application/json");
        assert_eq!(body_of(&req), r#"{"a":"{{missing}}"}"#);
        assert_eq!(un, ["missing"]);
    }

    #[test]
    fn auth_and_explicit_header_override() {
        let env = Env::from([("tok".into(), "abc".into())]);
        let mut s = Scenario {
            url: "http://h/".into(),
            auth: Some(Auth::Bearer { token: "{{tok}}".into() }),
            ..Default::default()
        };
        let (req, _) = build(Path::new("."), &s, &env).unwrap();
        assert_eq!(req.headers()["authorization"], "Bearer abc");

        s.auth = Some(Auth::Basic { username: "u".into(), password: "p".into() });
        let (req, _) = build(Path::new("."), &s, &env).unwrap();
        assert_eq!(req.headers()["authorization"], "Basic dTpw");

        s.headers.insert("Authorization".into(), "Custom".into());
        let (req, _) = build(Path::new("."), &s, &env).unwrap();
        assert_eq!(req.headers()["authorization"], "Custom");
    }

    #[test]
    fn form_and_multipart_bodies() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("f.txt"), "FILEDATA").unwrap();
        let env = Env::from([("v".into(), "a b".into())]);
        let mut s = Scenario {
            method: "POST".into(),
            url: "http://h/".into(),
            body_type: Some(BodyType::Form),
            body: json!({"x": "{{v}}", "n": 1}),
            ..Default::default()
        };
        let (req, _) = build(dir.path(), &s, &env).unwrap();
        assert_eq!(req.headers()["content-type"], "application/x-www-form-urlencoded");
        assert_eq!(body_of(&req), "x=a+b&n=1");

        s.body_type = Some(BodyType::Multipart);
        s.body = json!({"x": "{{v}}", "file": "@f.txt"});
        let (req, _) = build(dir.path(), &s, &env).unwrap();
        assert!(req.headers()["content-type"].to_str().unwrap().starts_with("multipart/form-data"));

        s.body = json!({"file": "@missing.txt"});
        assert!(matches!(build(dir.path(), &s, &env), Err(Error::AttachmentUnreadable { .. })));

        s.body = json!([1]);
        assert!(matches!(build(dir.path(), &s, &env), Err(Error::BodyNotAnObject("multipart"))));
    }

    #[test]
    fn rejects_bad_method_and_header() {
        let env = Env::new();
        let s = Scenario { method: "BAD METHOD".into(), url: "http://h/".into(), ..Default::default() };
        assert!(matches!(build(Path::new("."), &s, &env), Err(Error::InvalidMethod(_))));
        let s = Scenario {
            url: "http://h/".into(),
            headers: BTreeMap::from([("bad name".into(), "x".into())]),
            ..Default::default()
        };
        assert!(matches!(build(Path::new("."), &s, &env), Err(Error::InvalidHeaderName(_))));
    }
}
