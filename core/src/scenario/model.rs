use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const DEFAULT_METHOD: &str = "GET";

pub(crate) fn default_method() -> String {
    DEFAULT_METHOD.into()
}

/// One request, stored as a JSON file in the project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Scenario {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Kept in the file but not sent.
    #[serde(default, rename = "disabledHeaders", skip_serializing_if = "BTreeMap::is_empty")]
    pub disabled_headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    /// How to interpret `body`; inferred from its JSON type when absent.
    #[serde(default, rename = "bodyType", skip_serializing_if = "Option::is_none")]
    pub body_type: Option<BodyType>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = crate::specta_types::JsonValue))]
    pub body: Value,
    /// Env var name -> `$.json.path` or `header:Name`, saved after each send.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub capture: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect: Option<Expect>,
    /// Keys we don't know about, preserved so hand edits survive a save.
    /// Hidden from TypeScript: the UI passes them through untouched.
    #[serde(flatten)]
    #[cfg_attr(feature = "specta", specta(skip))]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum BodyType {
    None,
    Json,
    Text,
    /// `body` is an object, sent as `application/x-www-form-urlencoded`.
    Form,
    /// `body` is an object; values starting with `@` are file paths.
    Multipart,
}

impl BodyType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Json => "json",
            Self::Text => "text",
            Self::Form => "form",
            Self::Multipart => "multipart",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Auth {
    Bearer {
        #[serde(default)]
        token: String,
    },
    Basic {
        #[serde(default)]
        username: String,
        #[serde(default)]
        password: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Expect {
    /// A single status or a list of accepted statuses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusExpect>,
    /// `$.json.path` -> expected value.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "specta", specta(type = BTreeMap<String, crate::specta_types::JsonValue>))]
    pub json: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum StatusExpect {
    One(u16),
    Any(Vec<u16>),
}

impl StatusExpect {
    pub fn accepts(&self, status: u16) -> bool {
        match self {
            Self::One(expected) => *expected == status,
            Self::Any(accepted) => accepted.contains(&status),
        }
    }

    /// `"200"` or `"200 | 201"`.
    pub fn describe(&self) -> String {
        match self {
            Self::One(expected) => expected.to_string(),
            Self::Any(accepted) => accepted.iter().map(u16::to_string).collect::<Vec<_>>().join(" | "),
        }
    }
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            title: String::new(),
            description: String::new(),
            method: default_method(),
            url: String::new(),
            headers: BTreeMap::new(),
            disabled_headers: BTreeMap::new(),
            auth: None,
            body_type: None,
            body: Value::Null,
            capture: BTreeMap::new(),
            expect: None,
            extra: Map::new(),
        }
    }
}

impl Scenario {
    pub fn titled(title: impl Into<String>) -> Self {
        Self { title: title.into(), ..Self::default() }
    }

    pub fn effective_body_type(&self) -> BodyType {
        self.body_type.unwrap_or(match &self.body {
            Value::Null => BodyType::None,
            Value::String(_) => BodyType::Text,
            _ => BodyType::Json,
        })
    }

    pub fn has_status_expectation(&self) -> bool {
        self.expect.as_ref().is_some_and(|e| e.status.is_some())
    }
}
