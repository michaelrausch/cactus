//! One request file: an OpenCollection `HttpRequest`, reduced to what cactus
//! edits. Saving patches the file in place, so anything else in it (tags,
//! examples, settings cactus ignores, comments) stays as written.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::collection::model::{LenientString, is_false, is_selected_json, selected_or_first};
use crate::collection::{Auth, Header, Inherit, Variable};

pub const DEFAULT_METHOD: &str = "GET";

pub(crate) fn default_method() -> LenientString {
    DEFAULT_METHOD.into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Scenario {
    #[serde(default)]
    pub info: RequestInfo,
    #[serde(default)]
    pub http: Http,
    #[serde(default, skip_serializing_if = "Runtime::is_empty")]
    pub runtime: Runtime,
    #[serde(default, skip_serializing_if = "Settings::is_empty")]
    pub settings: Settings,
    #[serde(default, skip_serializing_if = "LenientString::is_empty")]
    pub docs: LenientString,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum RequestType {
    #[default]
    Http,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RequestInfo {
    #[serde(default)]
    pub name: LenientString,
    #[serde(rename = "type", default)]
    pub kind: RequestType,
    /// Position among its siblings; Bruno lists and runs requests by it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<f64>,
    /// OpenCollection tags; `auth` marks a request that logs in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<LenientString>,
}

/// The tag that marks a request as one that logs in.
pub const AUTH_TAG: &str = "auth";

impl RequestInfo {
    pub fn is_auth(&self) -> bool {
        self.tags.iter().any(|tag| tag == AUTH_TAG)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Http {
    #[serde(default = "default_method")]
    pub method: LenientString,
    #[serde(default)]
    pub url: LenientString,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<Header>,
    /// Query params mirror the URL, which is what cactus edits and sends;
    /// path params fill `:name` segments of it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Param>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
    /// `None` sends no auth; `inherit` takes the folder's or collection's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
}

impl Default for Http {
    fn default() -> Self {
        Self {
            method: default_method(),
            url: LenientString::default(),
            headers: Vec::new(),
            params: Vec::new(),
            body: None,
            auth: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ParamKind {
    Query,
    Path,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Param {
    #[serde(default)]
    pub name: LenientString,
    #[serde(default)]
    pub value: LenientString,
    #[serde(rename = "type")]
    pub kind: ParamKind,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
}

/// `name=value` of a form body.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FormField {
    #[serde(default)]
    pub name: LenientString,
    #[serde(default)]
    pub value: LenientString,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum PartKind {
    #[default]
    Text,
    File,
}

/// A file part's value is a path relative to the project, or several.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum PartValue {
    One(String),
    Many(Vec<String>),
}

impl Default for PartValue {
    fn default() -> Self {
        Self::One(String::new())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MultipartField {
    #[serde(default)]
    pub name: LenientString,
    #[serde(rename = "type", default)]
    pub kind: PartKind,
    #[serde(default)]
    pub value: PartValue,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FileVariant {
    #[serde(default)]
    pub file_path: String,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub selected: bool,
}

/// The body kinds cactus sends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum TypedBody {
    /// JSON text, sent as written after `{{var}}` substitution.
    Json {
        #[serde(default)]
        data: LenientString,
    },
    Text {
        #[serde(default)]
        data: LenientString,
    },
    Xml {
        #[serde(default)]
        data: LenientString,
    },
    Sparql {
        #[serde(default)]
        data: LenientString,
    },
    FormUrlencoded {
        #[serde(default)]
        data: Vec<FormField>,
    },
    MultipartForm {
        #[serde(default)]
        data: Vec<MultipartField>,
    },
    File {
        #[serde(default)]
        data: Vec<FileVariant>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Body {
    Typed(TypedBody),
    /// Body variants or a shape cactus doesn't edit, kept as written. The
    /// selected variant (or else the first) is what gets sent.
    Other(#[cfg_attr(feature = "specta", specta(type = crate::specta_types::JsonValue))] Value),
}

impl Body {
    /// The body to send.
    pub fn effective(&self) -> Option<TypedBody> {
        match self {
            Self::Typed(body) => Some(body.clone()),
            Self::Other(Value::Array(variants)) => {
                let chosen = selected_or_first(variants, is_selected_json)?;
                serde_json::from_value(chosen["body"].clone()).ok()
            }
            Self::Other(_) => None,
        }
    }
}

/// A test run against each response, e.g. `res.status eq 200`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Assertion {
    #[serde(default)]
    pub expression: LenientString,
    #[serde(default)]
    pub operator: LenientString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<LenientString>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
}

/// The one action type cactus runs. An enum, not a string, so that any
/// other type fails to match and lands in [`Action::Other`].
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum SetVariableType {
    #[default]
    #[serde(rename = "set-variable")]
    SetVariable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ActionPhase {
    BeforeRequest,
    AfterResponse,
}

/// What a set-variable action reads. `method` is kept for Bruno; cactus
/// ignores it and evaluates every `expression` as a `res.…` path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Selector {
    #[serde(default)]
    pub expression: LenientString,
    #[serde(default = "jsonq")]
    pub method: String,
}

pub(crate) fn jsonq() -> String {
    "jsonq".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ActionVariable {
    #[serde(default)]
    pub name: LenientString,
    #[serde(default = "runtime_scope")]
    pub scope: String,
}

pub(crate) fn runtime_scope() -> String {
    "runtime".into()
}

/// Save a value from the response as a variable for later requests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SetVariable {
    #[serde(rename = "type")]
    pub kind: SetVariableType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<ActionPhase>,
    pub selector: Selector,
    pub variable: ActionVariable,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
}

impl SetVariable {
    /// After each response, store what `expression` selects as `name`.
    pub fn new(name: impl Into<String>, expression: impl Into<String>) -> Self {
        Self {
            kind: SetVariableType::SetVariable,
            phase: Some(ActionPhase::AfterResponse),
            selector: Selector { expression: LenientString(expression.into()), method: jsonq() },
            variable: ActionVariable { name: LenientString(name.into()), scope: runtime_scope() },
            disabled: false,
        }
    }
}

/// Actions cactus doesn't run are kept as written, so saving won't drop them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Action {
    SetVariable(SetVariable),
    Other(#[cfg_attr(feature = "specta", specta(type = crate::specta_types::JsonValue))] Value),
}

/// JavaScript run around the request. Kept, but cactus has no engine to run it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Script {
    #[serde(rename = "type", default)]
    pub kind: LenientString,
    #[serde(default)]
    pub code: LenientString,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Runtime {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Variable>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<Script>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<Assertion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

impl Runtime {
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty() && self.scripts.is_empty() && self.assertions.is_empty() && self.actions.is_empty()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Timeout {
    // A JS `number`, accepting precision loss: timeouts stay far below 2^53.
    Millis(#[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))] u64),
    Inherit(Inherit),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Timeout>,
}

impl Settings {
    pub fn is_empty(&self) -> bool {
        self.timeout.is_none()
    }
}

impl Scenario {
    /// A new GET request that inherits auth from its folder or collection.
    pub fn titled(title: impl Into<String>) -> Self {
        let mut scenario = Self::default();
        scenario.info.name = LenientString(title.into());
        scenario.http.auth = Some(Auth::INHERIT);
        scenario
    }

    pub fn has_scripts(&self) -> bool {
        !self.runtime.scripts.is_empty()
    }

    /// Rebuild the query params from the URL, which is what the editor
    /// changes, keeping path params and disabled query params where they are.
    pub fn sync_query_params(&mut self) {
        let mut from_url = query_pairs(&self.http.url).into_iter();
        let mut params = Vec::with_capacity(self.http.params.len());
        for param in self.http.params.drain(..) {
            if param.kind == ParamKind::Path || param.disabled {
                params.push(param);
            } else if let Some((name, value)) = from_url.next() {
                params.push(Param { name: name.into(), value: value.into(), ..param });
            }
        }
        params.extend(from_url.map(|(name, value)| Param {
            name: name.into(),
            value: value.into(),
            kind: ParamKind::Query,
            disabled: false,
        }));
        self.http.params = params;
    }
}

/// `?a=1&b` → `[("a", "1"), ("b", "")]`, nothing decoded, so `{{vars}}`
/// and encoded values stay exactly as typed.
fn query_pairs(url: &str) -> Vec<(String, String)> {
    let without_hash = url.split_once('#').map_or(url, |(before, _)| before);
    let Some((_, query)) = without_hash.split_once('?') else { return Vec::new() };
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (name.to_string(), value.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_bruno_request() {
        let yaml = r#"
info:
  name: Create user
  type: http
  seq: 1
http:
  method: POST
  url: https://api.example.com/users
  body:
    type: json
    data: |-
      {"name": "x"}
  auth: inherit
runtime:
  scripts:
    - type: tests
      code: test("201", () => expect(res.status).to.equal(201));
settings:
  encodeUrl: true
"#;
        let s: Scenario = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(s.info.seq, Some(1.0));
        assert_eq!(s.http.auth, Some(Auth::INHERIT));
        assert_eq!(s.http.body, Some(Body::Typed(TypedBody::Json { data: r#"{"name": "x"}"#.into() })));
        assert!(s.has_scripts());
        assert!(s.settings.is_empty());
    }

    #[test]
    fn body_variants_send_the_selected_one() {
        let yaml =
            "- {title: A, body: {type: text, data: a}}\n- {title: B, selected: true, body: {type: text, data: b}}";
        let body: Body = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(matches!(body, Body::Other(_)));
        assert_eq!(body.effective(), Some(TypedBody::Text { data: "b".into() }));
    }

    /// The same cases run against the editor's `withUrl` (ui/src/domain/scenario.test.ts),
    /// which has to keep params exactly as a save writes them.
    #[test]
    fn query_params_match_the_shared_cases() {
        #[derive(serde::Deserialize)]
        struct SharedCase {
            case: String,
            url: String,
            params: Vec<Param>,
            expected: Vec<Param>,
        }
        let cases: Vec<SharedCase> =
            serde_json::from_str(include_str!("../../tests/fixtures/query-params.json")).unwrap();
        for case in cases {
            let mut s = Scenario::default();
            s.http.url = case.url.into();
            s.http.params = case.params;
            s.sync_query_params();
            assert_eq!(s.http.params, case.expected, "{}", case.case);
        }
    }

    #[test]
    fn query_params_follow_the_url() {
        let mut s = Scenario::default();
        s.http.url = "http://h/:id?a=1&b={{x}}#top".into();
        s.http.params = vec![
            Param { name: "id".into(), value: "7".into(), kind: ParamKind::Path, disabled: false },
            Param { name: "old".into(), value: "0".into(), kind: ParamKind::Query, disabled: false },
            Param { name: "off".into(), value: "1".into(), kind: ParamKind::Query, disabled: true },
        ];
        s.sync_query_params();
        let names: Vec<_> = s.http.params.iter().map(|p| (p.name.as_str(), p.value.as_str())).collect();
        assert_eq!(names, [("id", "7"), ("a", "1"), ("off", "1"), ("b", "{{x}}")]);
    }
}
