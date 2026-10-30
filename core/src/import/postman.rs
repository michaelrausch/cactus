//! Postman collection v2.x.

use serde_json::Value;

use super::{ImportedScenario, ParsedCollection, child_dir, description_text};
use crate::collection::{ApiKeyPlacement, Auth, AuthMode, Header};
use crate::scenario::{
    Body, DEFAULT_METHOD, FormField, MultipartField, PartKind, PartValue, Scenario, Script, TypedBody,
};

pub(super) fn parse(document: &Value) -> ParsedCollection {
    let mut scenarios = Vec::new();
    collect_items(as_slice(&document["item"]), "", &mut scenarios);
    let variables = document["variable"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|var| Some((var["key"].as_str()?.to_string(), description_text(&var["value"]))))
        .collect();
    ParsedCollection { scenarios, variables }
}

fn as_slice(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

/// Folders (items with `item`) become directories; requests become files.
fn collect_items(items: &[Value], dir: &str, out: &mut Vec<ImportedScenario>) {
    for item in items {
        let name = item["name"].as_str().unwrap_or_default();
        if let Some(children) = item["item"].as_array() {
            collect_items(children, &child_dir(dir, name), out);
        } else if !item["request"].is_null() {
            out.push(ImportedScenario { dir: dir.to_string(), scenario: parse_request(name, item) });
        }
    }
}

fn parse_request(name: &str, item: &Value) -> Scenario {
    let request = &item["request"];
    let mut scenario = Scenario::titled(name);
    // A request may be just a URL string.
    if let Some(url) = request.as_str() {
        scenario.http.url = url.to_string();
        return scenario;
    }
    scenario.http.method = request["method"].as_str().unwrap_or(DEFAULT_METHOD).to_uppercase();
    scenario.http.url = parse_url(&request["url"]);
    scenario.http.headers = key_values(&request["header"])
        .map(|field| Header { name: field.key.into(), value: description_text(field.value), disabled: field.disabled })
        .collect();
    scenario.http.body = parse_body(&request["body"]);
    scenario.http.auth = parse_auth(&request["auth"]);
    scenario.docs = parse_description(item, request);
    scenario.runtime.scripts = parse_scripts(&item["event"]);
    scenario
}

/// `url` is either a string or an object with a `raw` string.
fn parse_url(url: &Value) -> String {
    url.as_str().or_else(|| url["raw"].as_str()).unwrap_or_default().to_string()
}

/// The item's description wins over the request's.
fn parse_description(item: &Value, request: &Value) -> String {
    [description_text(&item["description"]), description_text(&request["description"])]
        .into_iter()
        .find(|d| !d.is_empty())
        .unwrap_or_default()
}

fn parse_body(body: &Value) -> Option<Body> {
    let typed = match body["mode"].as_str()? {
        "raw" => raw_body(body)?,
        "urlencoded" => TypedBody::FormUrlencoded {
            data: key_values(&body["urlencoded"])
                .map(|f| FormField { name: f.key.into(), value: description_text(f.value), disabled: f.disabled })
                .collect(),
        },
        "formdata" => TypedBody::MultipartForm { data: form_data(&body["formdata"]) },
        _ => return None,
    };
    Some(Body::Typed(typed))
}

/// Raw bodies are kept as written, typed by their language.
fn raw_body(body: &Value) -> Option<TypedBody> {
    let data = body["raw"].as_str().filter(|raw| !raw.is_empty())?.to_string();
    Some(match body["options"]["raw"]["language"].as_str() {
        Some("json") => TypedBody::Json { data },
        Some("xml") => TypedBody::Xml { data },
        _ => TypedBody::Text { data },
    })
}

/// File fields keep their `src` path, or paths.
fn form_data(fields: &Value) -> Vec<MultipartField> {
    as_slice(fields)
        .iter()
        .filter_map(|field| {
            let name = field["key"].as_str()?.to_string();
            let disabled = field["disabled"].as_bool().unwrap_or(false);
            let (kind, value) = if field["type"].as_str() == Some("file") {
                let value = match &field["src"] {
                    Value::Array(paths) => PartValue::Many(paths.iter().map(description_text).collect()),
                    src => PartValue::One(description_text(src)),
                };
                (PartKind::File, value)
            } else {
                (PartKind::Text, PartValue::One(description_text(&field["value"])))
            };
            Some(MultipartField { name, kind, value, disabled })
        })
        .collect()
}

/// Postman inherits auth unless a request says otherwise. Settings live in
/// a key/value list named after the auth type.
fn parse_auth(auth: &Value) -> Option<Auth> {
    let Some(auth_type) = auth["type"].as_str() else { return Some(Auth::INHERIT) };
    let setting =
        |name: &str| key_values(&auth[auth_type]).find(|f| f.key == name).map_or_default(|f| description_text(f.value));
    let mode = match auth_type {
        "bearer" => AuthMode::Bearer { token: setting("token") },
        "basic" => AuthMode::Basic { username: setting("username"), password: setting("password") },
        "apikey" => AuthMode::ApiKey {
            key: setting("key"),
            value: setting("value"),
            placement: Some(if setting("in") == "query" { ApiKeyPlacement::Query } else { ApiKeyPlacement::Header }),
        },
        "noauth" => return None,
        _ => return Some(Auth::INHERIT),
    };
    Some(Auth::Mode(mode))
}

/// Test and pre-request scripts are kept for Bruno; cactus doesn't run them.
fn parse_scripts(events: &Value) -> Vec<Script> {
    as_slice(events)
        .iter()
        .filter_map(|event| {
            let kind = match event["listen"].as_str()? {
                "test" => "tests",
                "prerequest" => "before-request",
                _ => return None,
            };
            let code = match &event["script"]["exec"] {
                Value::Array(lines) => lines.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n"),
                other => description_text(other),
            };
            (!code.trim().is_empty()).then(|| Script { kind: kind.into(), code })
        })
        .collect()
}

/// One entry of Postman's `[{key, value, disabled}]` lists.
struct KeyValue<'a> {
    key: &'a str,
    value: &'a Value,
    disabled: bool,
}

fn key_values(list: &Value) -> impl Iterator<Item = KeyValue<'_>> {
    list.as_array().into_iter().flatten().filter_map(|entry| {
        Some(KeyValue {
            key: entry["key"].as_str()?,
            value: &entry["value"],
            disabled: entry["disabled"].as_bool().unwrap_or(false),
        })
    })
}
