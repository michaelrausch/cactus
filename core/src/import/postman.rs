//! Postman collection v2.x.

use serde_json::{Map, Value};

use super::{ImportedScenario, ParsedCollection, child_dir, description_text};
use crate::http::ATTACHMENT_PREFIX;
use crate::scenario::{Auth, BodyType, DEFAULT_METHOD, Scenario};

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

/// Folders (items with `item`) become directories; requests become scenarios.
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
    // A request may be just a URL string.
    if let Some(url) = request.as_str() {
        return Scenario { url: url.to_string(), ..Scenario::titled(name) };
    }
    let mut scenario = Scenario {
        method: request["method"].as_str().unwrap_or(DEFAULT_METHOD).to_uppercase(),
        url: parse_url(&request["url"]),
        description: parse_description(item, request),
        auth: parse_auth(&request["auth"]),
        ..Scenario::titled(name)
    };
    apply_headers(&mut scenario, &request["header"]);
    apply_body(&mut scenario, &request["body"]);
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

fn apply_headers(scenario: &mut Scenario, headers: &Value) {
    for field in key_values(headers) {
        let target = if field.disabled { &mut scenario.disabled_headers } else { &mut scenario.headers };
        target.insert(field.key.to_string(), description_text(field.value));
    }
}

fn apply_body(scenario: &mut Scenario, body: &Value) {
    match body["mode"].as_str() {
        Some("raw") => scenario.body = raw_body(body),
        Some("urlencoded") => {
            scenario.body_type = Some(BodyType::Form);
            scenario.body = field_body(&body["urlencoded"]);
        }
        Some("formdata") => {
            scenario.body_type = Some(BodyType::Multipart);
            scenario.body = field_body(&body["formdata"]);
        }
        _ => {}
    }
}

/// Raw bodies marked as JSON are stored as JSON when they parse; other raw
/// bodies stay text.
fn raw_body(body: &Value) -> Value {
    let raw = body["raw"].as_str().unwrap_or_default();
    let is_json = body["options"]["raw"]["language"].as_str() == Some("json");
    match serde_json::from_str::<Value>(raw) {
        Ok(parsed) if is_json => parsed,
        _ if raw.is_empty() => Value::Null,
        _ => Value::String(raw.to_string()),
    }
}

/// Enabled form fields as an object; file fields become `@path` values.
fn field_body(fields: &Value) -> Value {
    let mut object = Map::new();
    for field in fields.as_array().into_iter().flatten() {
        let Some(key) = field["key"].as_str() else { continue };
        if field["disabled"].as_bool() == Some(true) {
            continue;
        }
        let value = if field["type"].as_str() == Some("file") {
            format!("{ATTACHMENT_PREFIX}{}", field["src"].as_str().unwrap_or_default())
        } else {
            description_text(&field["value"])
        };
        object.insert(key.to_string(), Value::String(value));
    }
    Value::Object(object)
}

/// Auth settings live in a key/value list named after the auth type.
fn parse_auth(auth: &Value) -> Option<Auth> {
    let auth_type = auth["type"].as_str()?;
    let setting =
        |name: &str| key_values(&auth[auth_type]).find(|f| f.key == name).map_or_default(|f| description_text(f.value));
    match auth_type {
        "bearer" => Some(Auth::Bearer { token: setting("token") }),
        "basic" => Some(Auth::Basic { username: setting("username"), password: setting("password") }),
        _ => None,
    }
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
