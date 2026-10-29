//! OpenAPI 3 and Swagger 2.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use super::{ImportedScenario, ParsedCollection, child_dir, description_text};
use crate::scenario::{BodyType, Scenario};

const HTTP_METHODS: [&str; 7] = ["get", "post", "put", "patch", "delete", "head", "options"];
/// Imported URLs are written as `{{baseUrl}}/path` and the server URL goes
/// into `env.json` under this name.
const BASE_URL_VARIABLE: &str = "baseUrl";
const DEFAULT_SWAGGER_SCHEME: &str = "https";
const JSON_MEDIA_TYPE: &str = "application/json";
const FORM_MEDIA_TYPE: &str = "application/x-www-form-urlencoded";

pub(super) fn parse(document: &Value) -> ParsedCollection {
    let scenarios = operations(document).map(|(path, method, op)| operation_to_scenario(path, method, op)).collect();
    let variables = base_url(document).map_or_default(|url| BTreeMap::from([(BASE_URL_VARIABLE.to_string(), url)]));
    ParsedCollection { scenarios, variables }
}

/// `(path, method, operation)` for every operation, in document order.
fn operations(document: &Value) -> impl Iterator<Item = (&str, &'static str, &Value)> {
    document["paths"].as_object().into_iter().flatten().flat_map(|(path, item)| {
        HTTP_METHODS.into_iter().filter_map(move |method| {
            item.get(method).filter(|op| op.is_object()).map(|op| (path.as_str(), method, op))
        })
    })
}

fn base_url(document: &Value) -> Option<String> {
    document["servers"][0]["url"].as_str().map(str::to_string).or_else(|| swagger2_base_url(document))
}

fn swagger2_base_url(document: &Value) -> Option<String> {
    let host = document["host"].as_str()?;
    let scheme = document["schemes"][0].as_str().unwrap_or(DEFAULT_SWAGGER_SCHEME);
    let base_path = document["basePath"].as_str().unwrap_or_default();
    Some(format!("{scheme}://{host}{base_path}"))
}

/// Operations are grouped into folders by their first tag.
fn operation_to_scenario(path: &str, method: &str, operation: &Value) -> ImportedScenario {
    let mut scenario = Scenario {
        title: operation_title(path, method, operation),
        description: description_text(&operation["description"]),
        method: method.to_uppercase(),
        url: templated_url(path),
        ..Scenario::default()
    };
    apply_request_body(&mut scenario, &operation["requestBody"]["content"]);
    let dir = operation["tags"][0].as_str().map_or_else(String::new, |tag| child_dir("", tag));
    ImportedScenario { dir, scenario }
}

fn operation_title(path: &str, method: &str, operation: &Value) -> String {
    operation["summary"]
        .as_str()
        .or_else(|| operation["operationId"].as_str())
        .map_or_else(|| format!("{} {path}", method.to_uppercase()), str::to_string)
}

/// `/pets/{id}` → `{{baseUrl}}/pets/{{id}}`.
fn templated_url(path: &str) -> String {
    format!("{{{{{BASE_URL_VARIABLE}}}}}{}", path.replace('{', "{{").replace('}', "}}"))
}

fn apply_request_body(scenario: &mut Scenario, content: &Value) {
    if let Some(media) = content.get(JSON_MEDIA_TYPE) {
        scenario.body = json_example(media);
    } else if let Some(media) = content.get(FORM_MEDIA_TYPE) {
        scenario.body_type = Some(BodyType::Form);
        scenario.body = empty_form_fields(media);
    }
}

/// The inline example, else the first named example, else `{}`.
fn json_example(media: &Value) -> Value {
    [media["example"].clone(), first_named_example(&media["examples"])]
        .into_iter()
        .find(|example| !example.is_null())
        .unwrap_or_else(|| json!({}))
}

fn first_named_example(examples: &Value) -> Value {
    examples.as_object().and_then(|named| named.values().next()).map_or(Value::Null, |example| example["value"].clone())
}

/// One empty field per schema property.
fn empty_form_fields(media: &Value) -> Value {
    let fields: Map<String, Value> = media["schema"]["properties"]
        .as_object()
        .map_or_default(|properties| properties.keys().map(|k| (k.clone(), Value::String(String::new()))).collect());
    Value::Object(fields)
}
