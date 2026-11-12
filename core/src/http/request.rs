//! Turning a [`Scenario`] into an outgoing request.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, RequestBuilder, Url, multipart};

use crate::collection::model::selected_or_first;
use crate::collection::{ApiKeyPlacement, Auth, AuthMode, Header, RequestDefaults};
use crate::env::{Env, Substituter};
use crate::scenario::{
    Body, FileVariant, FormField, MultipartField, ParamKind, PartKind, PartValue, Scenario, Timeout, TypedBody,
};
use crate::{Error, ProjectPath, Result};

const JSON_CONTENT_TYPE: &str = "application/json";
const XML_CONTENT_TYPE: &str = "application/xml";
const SPARQL_CONTENT_TYPE: &str = "application/sparql-query";
const PATH_PARAM_PREFIX: char = ':';

pub struct PreparedRequest {
    pub request: reqwest::Request,
    /// `{{names}}` that no env layer defines, left as-is in the request.
    pub unresolved: Vec<String>,
    /// Parts of the request cactus could not send as written.
    pub warnings: Vec<String>,
}

/// Build the outgoing request with env vars applied. `inherited` is what the
/// collection and folders above the request set, outermost first (see
/// [`crate::collection::defaults_chain`]). `project` anchors relative file
/// paths.
pub fn build_request(
    client: &reqwest::Client,
    project: &Path,
    scenario: &Scenario,
    inherited: &[RequestDefaults],
    env: &Env,
) -> Result<PreparedRequest> {
    let mut vars = Substituter::new(env);
    let mut warnings = Vec::new();
    let http = &scenario.http;
    let url = fill_path_params(&vars.apply(http.url.trim()), scenario, &mut vars);
    let url = parse_url(&url, vars.unresolved())?;
    let method = parse_method(&http.method)?;
    let auth = effective_auth(http.auth.as_ref(), inherited);
    let mut headers = HeaderMap::new();
    let mut builder = client.request(method, url);
    if let Some(Auth::Mode(AuthMode::ApiKey { key, value, placement })) = auth {
        let (key, value) = (vars.apply(key.trim()), vars.apply(value));
        match placement {
            // `query` encodes the pair and keeps any `#fragment` last.
            Some(ApiKeyPlacement::Query) => builder = builder.query(&[(key, value)]),
            _ => set_header(&mut headers, &key, &value)?,
        }
    }
    // Auth goes first so an explicit Authorization header overrides it.
    let builder = apply_auth(builder, auth, &mut vars, &mut warnings);
    add_headers(&mut headers, inherited, &http.headers, &mut vars)?;
    let mut builder = attach_body(builder, &mut headers, project, http.body.as_ref(), &mut vars, &mut warnings)?;
    if let Some(Timeout::Millis(ms)) = scenario.settings.timeout {
        builder = builder.timeout(Duration::from_millis(ms));
    }
    if scenario.has_scripts() {
        warnings.push("scripts are kept in the file but cactus doesn't run them".into());
    }
    let request = builder.headers(headers).build()?;
    Ok(PreparedRequest { request, unresolved: vars.into_unresolved(), warnings })
}

/// The URL `scenario` will be sent to with `env`, without building the
/// request: for showing where a step is going before it gets there.
pub fn preview_url(scenario: &Scenario, env: &Env) -> String {
    let mut vars = Substituter::new(env);
    fill_path_params(&vars.apply(scenario.http.url.trim()), scenario, &mut vars)
}

/// Checked here so a blank or malformed URL reads as such rather than as
/// reqwest's bare "builder error". `unresolved` names the placeholders that
/// are likely why it does not parse.
fn parse_url(url: &str, unresolved: &[String]) -> Result<Url> {
    if url.is_empty() {
        return Err(Error::MissingUrl);
    }
    Url::parse(url).map_err(|e| {
        let reason = match unresolved {
            [] => e.to_string(),
            names => format!("{{{{{}}}}} not set", names.join("}}, {{")),
        };
        Error::InvalidUrl { url: url.to_string(), reason }
    })
}

fn parse_method(method: &str) -> Result<Method> {
    Method::from_bytes(method.trim().to_uppercase().as_bytes()).map_err(|_| Error::InvalidMethod(method.to_string()))
}

/// The request's own auth, or for `inherit` the nearest folder's or the
/// collection's. No `auth` at all means none.
fn effective_auth<'a>(own: Option<&'a Auth>, inherited: &'a [RequestDefaults]) -> Option<&'a Auth> {
    match own {
        Some(auth) if auth.is_inherit() => {
            inherited.iter().rev().filter_map(|defaults| defaults.auth.as_ref()).find(|auth| !auth.is_inherit())
        }
        other => other,
    }
}

fn apply_auth(
    builder: RequestBuilder,
    auth: Option<&Auth>,
    vars: &mut Substituter,
    warnings: &mut Vec<String>,
) -> RequestBuilder {
    match auth {
        Some(Auth::Mode(AuthMode::Bearer { token })) => builder.bearer_auth(vars.apply(token)),
        Some(Auth::Mode(AuthMode::Basic { username, password })) => {
            builder.basic_auth(vars.apply(username), Some(vars.apply(password)))
        }
        Some(Auth::Other(other)) => {
            let kind = other.get("type").and_then(|t| t.as_str()).unwrap_or("this");
            warnings.push(format!("{kind} auth isn't supported by cactus; sent without auth"));
            builder
        }
        Some(Auth::Mode(AuthMode::ApiKey { .. }) | Auth::Inherit(_)) | None => builder,
    }
}

/// Inherited headers, then the request's own. A folder's header replaces the
/// collection's of the same name and a request's replaces both, but a name
/// the request itself lists twice (`Cookie`, `Accept`) is sent twice.
fn add_headers(
    headers: &mut HeaderMap,
    inherited: &[RequestDefaults],
    own: &[Header],
    vars: &mut Substituter,
) -> Result<()> {
    for header in inherited.iter().flat_map(|defaults| enabled(&defaults.headers)) {
        set_header(headers, &vars.apply(header.name.trim()), &vars.apply(&header.value))?;
    }
    let mut seen = HashSet::new();
    for header in enabled(own) {
        let (name, value) = parse_header(&vars.apply(header.name.trim()), &vars.apply(&header.value))?;
        if seen.insert(name.clone()) {
            headers.insert(name, value);
        } else {
            headers.append(name, value);
        }
    }
    Ok(())
}

fn enabled(headers: &[Header]) -> impl Iterator<Item = &Header> {
    headers.iter().filter(|h| !h.disabled && !h.name.trim().is_empty())
}

fn set_header(headers: &mut HeaderMap, name: &str, value: &str) -> Result<()> {
    let (name, value) = parse_header(name, value)?;
    headers.insert(name, value);
    Ok(())
}

fn parse_header(name: &str, value: &str) -> Result<(HeaderName, HeaderValue)> {
    let header_name =
        HeaderName::from_bytes(name.as_bytes()).map_err(|_| Error::InvalidHeaderName(name.to_string()))?;
    let header_value = HeaderValue::from_str(value).map_err(|_| Error::InvalidHeaderValue(name.to_string()))?;
    Ok((header_name, header_value))
}

/// Replace `:name` path segments with the enabled path params' values.
fn fill_path_params(url: &str, scenario: &Scenario, vars: &mut Substituter) -> String {
    let path_params: Vec<_> = scenario
        .http
        .params
        .iter()
        .filter(|p| p.kind == ParamKind::Path && !p.disabled && !p.name.is_empty())
        .collect();
    if path_params.is_empty() {
        return url.to_string();
    }
    let split_at = url.find(['?', '#']).unwrap_or(url.len());
    let (path, rest) = url.split_at(split_at);
    let filled: Vec<String> = path
        .split('/')
        .map(|segment| {
            let param =
                segment.strip_prefix(PATH_PARAM_PREFIX).and_then(|name| path_params.iter().find(|p| p.name == name));
            param.map_or_else(|| segment.to_string(), |p| vars.apply(&p.value))
        })
        .collect();
    format!("{}{rest}", filled.join("/"))
}

fn attach_body(
    builder: RequestBuilder,
    headers: &mut HeaderMap,
    project: &Path,
    body: Option<&Body>,
    vars: &mut Substituter,
    warnings: &mut Vec<String>,
) -> Result<RequestBuilder> {
    let Some(body) = body else { return Ok(builder) };
    let Some(effective) = body.effective() else {
        warnings.push("this body shape isn't supported by cactus; sent without a body".into());
        return Ok(builder);
    };
    let mut default_type = |content_type: &'static str| {
        headers.entry(CONTENT_TYPE).or_insert(HeaderValue::from_static(content_type));
    };
    Ok(match &effective {
        TypedBody::Json { data } | TypedBody::Text { data } | TypedBody::Xml { data } | TypedBody::Sparql { data }
            if data.trim().is_empty() =>
        {
            builder
        }
        TypedBody::Json { data } => {
            default_type(JSON_CONTENT_TYPE);
            builder.body(vars.apply(data))
        }
        TypedBody::Xml { data } => {
            default_type(XML_CONTENT_TYPE);
            builder.body(vars.apply(data))
        }
        TypedBody::Sparql { data } => {
            default_type(SPARQL_CONTENT_TYPE);
            builder.body(vars.apply(data))
        }
        TypedBody::Text { data } => builder.body(vars.apply(data)),
        TypedBody::FormUrlencoded { data } => form_body(builder, data, vars),
        TypedBody::MultipartForm { data } => multipart_body(builder, project, data, vars)?,
        TypedBody::File { data } => file_body(builder, headers, project, data)?,
    })
}

fn form_body(builder: RequestBuilder, fields: &[FormField], vars: &mut Substituter) -> RequestBuilder {
    let pairs: Vec<(String, String)> = fields
        .iter()
        .filter(|f| !f.disabled && !f.name.trim().is_empty())
        .map(|f| (vars.apply(f.name.trim()), vars.apply(&f.value)))
        .collect();
    builder.form(&pairs)
}

fn multipart_body(
    builder: RequestBuilder,
    project: &Path,
    fields: &[MultipartField],
    vars: &mut Substituter,
) -> Result<RequestBuilder> {
    let mut form = multipart::Form::new();
    for field in fields.iter().filter(|f| !f.disabled && !f.name.trim().is_empty()) {
        let name = vars.apply(field.name.trim());
        let values = match &field.value {
            PartValue::One(value) => std::slice::from_ref(value),
            PartValue::Many(values) => values.as_slice(),
        };
        for value in values {
            let value = vars.apply(value);
            form = match field.kind {
                PartKind::File => form.part(name.clone(), read_attachment(project, &value)?),
                PartKind::Text => form.text(name.clone(), value),
            };
        }
    }
    Ok(builder.multipart(form))
}

/// The selected file (or else the first) is the whole body.
fn file_body(
    builder: RequestBuilder,
    headers: &mut HeaderMap,
    project: &Path,
    variants: &[FileVariant],
) -> Result<RequestBuilder> {
    let Some(chosen) = selected_or_first(variants, |v| v.selected) else { return Ok(builder) };
    let data = read_file_in_project(project, &chosen.file_path)?;
    if let Ok(content_type) = HeaderValue::from_str(&chosen.content_type)
        && !chosen.content_type.is_empty()
    {
        headers.entry(CONTENT_TYPE).or_insert(content_type);
    }
    Ok(builder.body(data))
}

fn read_attachment(project: &Path, file: &str) -> Result<multipart::Part> {
    let data = read_file_in_project(project, file)?;
    let file_name = file.rsplit(['/', '\\']).next().unwrap_or(file).to_string();
    Ok(multipart::Part::bytes(data).file_name(file_name))
}

/// Attachments are read from inside the project only. A request file may come
/// from anyone (a cloned repository, an import), so an absolute or `../` path
/// would let it upload any file on the machine when it is sent.
fn read_file_in_project(project: &Path, file: &str) -> Result<Vec<u8>> {
    let relative = ProjectPath::parse(file).map_err(|_| Error::AttachmentOutsideProject(file.to_string()))?;
    let path = relative.resolve(project);
    fs::read(&path).map_err(Error::file(&path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::{Header, Variable};
    use crate::http::build_client;
    use crate::scenario::{Param, Script};

    fn body_of(req: &reqwest::Request) -> String {
        String::from_utf8(req.body().unwrap().as_bytes().unwrap().to_vec()).unwrap()
    }

    fn build(project: &Path, s: &Scenario, inherited: &[RequestDefaults], env: &Env) -> Result<PreparedRequest> {
        build_request(&build_client(), project, s, inherited, env)
    }

    fn request(method: &str, url: &str) -> Scenario {
        let mut s = Scenario::default();
        s.http.method = method.into();
        s.http.url = url.into();
        s
    }

    #[test]
    fn applies_env_headers_and_json_body() {
        let env = Env::from([("base".into(), "http://localhost:1".into()), ("tok".into(), "abc".into())]);
        let mut s = request("post", "{{base}}/x");
        s.http.headers = vec![Header::new("X-Tok", "{{tok}}"), Header { disabled: true, ..Header::new("X-Off", "1") }];
        s.http.body = Some(Body::Typed(TypedBody::Json { data: r#"{"a": "{{missing}}", "n": {{num}}}"#.into() }));
        let prepared = build(Path::new("."), &s, &[], &env).unwrap();
        let req = &prepared.request;
        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.url().as_str(), "http://localhost:1/x");
        assert_eq!(req.headers()["x-tok"], "abc");
        assert!(req.headers().get("x-off").is_none());
        assert_eq!(req.headers()["content-type"], "application/json");
        assert_eq!(body_of(req), r#"{"a": "{{missing}}", "n": {{num}}}"#);
        assert_eq!(prepared.unresolved, ["missing", "num"]);
    }

    #[test]
    fn auth_inherits_and_explicit_header_overrides() {
        let env = Env::from([("tok".into(), "abc".into())]);
        let collection = RequestDefaults {
            headers: vec![Header::new("X-Root", "r"), Header::new("X-Over", "root")],
            auth: Some(Auth::Mode(AuthMode::Basic { username: "u".into(), password: "p".into() })),
            variables: vec![Variable::new("unused", "x")],
        };
        let folder = RequestDefaults {
            headers: vec![Header::new("X-Over", "folder")],
            auth: Some(Auth::Mode(AuthMode::Bearer { token: "{{tok}}".into() })),
            ..Default::default()
        };
        let mut s = request("GET", "http://h/");
        s.http.auth = Some(Auth::INHERIT);
        let req = build(Path::new("."), &s, &[collection.clone(), folder], &env).unwrap().request;
        assert_eq!(req.headers()["authorization"], "Bearer abc");
        assert_eq!(req.headers()["x-root"], "r");
        assert_eq!(req.headers()["x-over"], "folder");

        let req = build(Path::new("."), &s, &[collection], &env).unwrap().request;
        assert_eq!(req.headers()["authorization"], "Basic dTpw");

        s.http.auth = None;
        assert!(build(Path::new("."), &s, &[], &env).unwrap().request.headers().get("authorization").is_none());

        s.http.auth = Some(Auth::Mode(AuthMode::Bearer { token: "t".into() }));
        s.http.headers.push(Header::new("Authorization", "Custom"));
        assert_eq!(build(Path::new("."), &s, &[], &env).unwrap().request.headers()["authorization"], "Custom");
    }

    #[test]
    fn api_key_path_params_and_warnings() {
        let env = Env::from([("id".into(), "42".into())]);
        let mut s = request("GET", "http://h/users/:id/posts?x=1");
        s.http.params =
            vec![Param { name: "id".into(), value: "{{id}}".into(), kind: ParamKind::Path, disabled: false }];
        s.http.auth = Some(Auth::Mode(AuthMode::ApiKey {
            key: "api_key".into(),
            value: "k".into(),
            placement: Some(ApiKeyPlacement::Query),
        }));
        s.runtime.scripts.push(Script { kind: "tests".into(), code: "test()".into() });
        let prepared = build(Path::new("."), &s, &[], &env).unwrap();
        assert_eq!(prepared.request.url().as_str(), "http://h/users/42/posts?x=1&api_key=k");
        assert_eq!(prepared.warnings.len(), 1);

        s.http.auth = Some(Auth::Mode(AuthMode::ApiKey { key: "X-Key".into(), value: "k".into(), placement: None }));
        assert_eq!(build(Path::new("."), &s, &[], &env).unwrap().request.headers()["x-key"], "k");
    }

    #[test]
    fn api_key_query_is_encoded() {
        let mut s = request("GET", "http://h/x?a=1#top");
        s.http.auth = Some(Auth::Mode(AuthMode::ApiKey {
            key: "api key".into(),
            value: "a&b=c".into(),
            placement: Some(ApiKeyPlacement::Query),
        }));
        let req = build(Path::new("."), &s, &[], &Env::new()).unwrap().request;
        assert_eq!(req.url().as_str(), "http://h/x?a=1&api+key=a%26b%3Dc#top");
    }

    #[test]
    fn repeated_request_headers_are_all_sent() {
        let collection = RequestDefaults { headers: vec![Header::new("Accept", "text/plain")], ..Default::default() };
        let mut s = request("GET", "http://h/");
        s.http.headers = vec![Header::new("Accept", "application/json"), Header::new("Accept", "text/html")];
        let req = build(Path::new("."), &s, &[collection], &Env::new()).unwrap().request;
        let accept: Vec<_> = req.headers().get_all("accept").iter().collect();
        assert_eq!(accept, ["application/json", "text/html"]);
    }

    #[test]
    fn form_multipart_and_file_bodies() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("f.txt"), "FILEDATA").unwrap();
        let env = Env::from([("v".into(), "a b".into())]);
        let mut s = request("POST", "http://h/");
        let field = |name: &str, value: &str| FormField { name: name.into(), value: value.into(), disabled: false };
        s.http.body = Some(Body::Typed(TypedBody::FormUrlencoded {
            data: vec![field("x", "{{v}}"), field("n", "1"), FormField { disabled: true, ..field("off", "1") }],
        }));
        let req = build(dir.path(), &s, &[], &env).unwrap().request;
        assert_eq!(req.headers()["content-type"], "application/x-www-form-urlencoded");
        assert_eq!(body_of(&req), "x=a+b&n=1");

        let part = |name: &str, kind: PartKind, value: &str| MultipartField {
            name: name.into(),
            kind,
            value: PartValue::One(value.into()),
            disabled: false,
        };
        s.http.body = Some(Body::Typed(TypedBody::MultipartForm {
            data: vec![part("x", PartKind::Text, "{{v}}"), part("file", PartKind::File, "f.txt")],
        }));
        let req = build(dir.path(), &s, &[], &env).unwrap().request;
        assert!(req.headers()["content-type"].to_str().unwrap().starts_with("multipart/form-data"));

        s.http.body =
            Some(Body::Typed(TypedBody::MultipartForm { data: vec![part("f", PartKind::File, "missing.txt")] }));
        assert!(matches!(build(dir.path(), &s, &[], &env), Err(Error::File { .. })));
        for outside in ["/etc/hosts", "../f.txt", "{{dir}}/../f.txt"] {
            s.http.body =
                Some(Body::Typed(TypedBody::MultipartForm { data: vec![part("f", PartKind::File, outside)] }));
            assert!(matches!(build(dir.path(), &s, &[], &env), Err(Error::AttachmentOutsideProject(_))), "{outside}");
        }

        s.http.body = Some(Body::Typed(TypedBody::File {
            data: vec![FileVariant { file_path: "f.txt".into(), content_type: "text/plain".into(), selected: true }],
        }));
        let req = build(dir.path(), &s, &[], &env).unwrap().request;
        assert_eq!(body_of(&req), "FILEDATA");
        assert_eq!(req.headers()["content-type"], "text/plain");
    }

    #[test]
    fn rejects_bad_method_and_header() {
        let env = Env::new();
        let s = request("BAD METHOD", "http://h/");
        assert!(matches!(build(Path::new("."), &s, &[], &env), Err(Error::InvalidMethod(_))));
        let mut s = request("GET", "http://h/");
        s.http.headers.push(Header::new("bad name", "x"));
        assert!(matches!(build(Path::new("."), &s, &[], &env), Err(Error::InvalidHeaderName(_))));
    }

    #[test]
    fn rejects_missing_and_invalid_urls() {
        let env = Env::new();
        let err = |url: &str| build(Path::new("."), &request("GET", url), &[], &env).err().unwrap().to_string();
        assert_eq!(err("  "), "the request has no URL");
        assert_eq!(err("{{base}}/x"), r#"invalid URL "{{base}}/x": {{base}} not set"#);
        assert!(err("localhost/x").starts_with(r#"invalid URL "localhost/x": "#));
    }
}
