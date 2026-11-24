//! OpenCollection v1.0.0 types shared by every file in a project, reduced to
//! what cactus reads or edits.
//!
//! Nothing here denies unknown keys and nothing collects them either: a save
//! patches the file in place (see [`super::patch`]), so keys these types
//! don't know stay in the file untouched, comments included. Shapes cactus
//! can't edit are kept as an `Other` JSON value so they survive a round trip
//! through the UI.

use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::text::plain_string;

/// The spec version written into new `opencollection.yml` files.
pub const SPEC_VERSION: &str = "1.0.0";

// serde's `skip_serializing_if` passes a reference.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) fn is_false(value: &bool) -> bool {
    !*value
}

/// Of a list of variants (of a body, a value, a file), the one marked
/// selected, or else the first.
pub(crate) fn selected_or_first<T>(variants: &[T], is_selected: impl Fn(&T) -> bool) -> Option<&T> {
    variants.iter().find(|variant| is_selected(variant)).or(variants.first())
}

/// Whether a variant written as JSON has `selected: true`.
pub(crate) fn is_selected_json(variant: &Value) -> bool {
    variant["selected"] == Value::Bool(true)
}

/// A string field that tolerates hand-written YAML such as `value: 8080` or
/// `value: true`, which YAML reads as a number or a boolean. It reads and
/// writes as a plain string, and derefs to [`String`].
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "specta", derive(specta::Type), specta(transparent))]
pub struct LenientString(pub String);

impl LenientString {
    /// For `skip_serializing_if`, which can't reach `str::is_empty` through `Deref`.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'de> Deserialize<'de> for LenientString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(plain_string(&Value::deserialize(deserializer)?)))
    }
}

impl Deref for LenientString {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
}

impl fmt::Display for LenientString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for LenientString {
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl From<&str> for LenientString {
    fn from(text: &str) -> Self {
        Self(text.to_string())
    }
}

impl From<&String> for LenientString {
    fn from(text: &String) -> Self {
        Self(text.clone())
    }
}

impl PartialEq<str> for LenientString {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for LenientString {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for LenientString {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

/// `name: value` sent with a request; `disabled` ones stay in the file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Header {
    #[serde(default)]
    pub name: LenientString,
    #[serde(default)]
    pub value: LenientString,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
}

impl Header {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self { name: LenientString(name.into()), value: LenientString(value.into()), disabled: false }
    }
}

/// The literal `inherit`: take the setting from the folder or collection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Inherit {
    Inherit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ApiKeyPlacement {
    Header,
    Query,
}

/// The auth modes cactus can send.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum AuthMode {
    Bearer {
        #[serde(default)]
        token: LenientString,
    },
    Basic {
        #[serde(default)]
        username: LenientString,
        #[serde(default)]
        password: LenientString,
    },
    #[serde(rename = "apikey")]
    ApiKey {
        #[serde(default)]
        key: LenientString,
        #[serde(default)]
        value: LenientString,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        placement: Option<ApiKeyPlacement>,
    },
}

/// `auth:` on a request, folder or collection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Auth {
    Inherit(Inherit),
    Mode(AuthMode),
    /// A mode cactus doesn't send (OAuth, AWS, digest, ...), kept as written.
    Other(#[cfg_attr(feature = "specta", specta(type = crate::specta_types::JsonValue))] Value),
}

impl Auth {
    pub const INHERIT: Self = Self::Inherit(Inherit::Inherit);

    pub fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit(_))
    }
}

/// A variable's value: a string, `{type, data}`, or variants cactus keeps as written.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum VariableValue {
    Text(String),
    Typed {
        #[serde(rename = "type", default)]
        kind: String,
        #[serde(default)]
        data: LenientString,
    },
    Other(#[cfg_attr(feature = "specta", specta(type = crate::specta_types::JsonValue))] Value),
}

impl VariableValue {
    /// The text substituted for `{{name}}`. Of a list of variants, the
    /// selected one (or else the first) is used.
    pub fn resolved(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Typed { data, .. } => data.to_string(),
            Self::Other(Value::Array(variants)) => selected_or_first(variants, is_selected_json)
                .and_then(|v| serde_json::from_value::<Self>(v["value"].clone()).ok())
                .map_or_else(String::new, |value| value.resolved()),
            Self::Other(other) => plain_string(other),
        }
    }
}

/// A variable in a collection, folder, environment or request. A `secret`
/// one has no value in the file: it comes from `.cactus/secrets.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Variable {
    #[serde(default)]
    pub name: LenientString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<VariableValue>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
}

impl Variable {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self { name: LenientString(name.into()), value: Some(VariableValue::Text(value.into())), ..Self::default() }
    }

    /// Whether this variable takes part in substitution.
    pub fn is_active(&self) -> bool {
        !self.disabled && !self.secret && !self.name.trim().is_empty()
    }

    pub fn resolved_value(&self) -> String {
        self.value.as_ref().map_or_else(String::new, VariableValue::resolved)
    }
}

/// `request:` in `opencollection.yml` and `folder.yml`: what every request
/// below inherits.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RequestDefaults {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<Header>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Variable>,
}

impl RequestDefaults {
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty() && self.auth.is_none() && self.variables.is_empty()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CollectionInfo {
    #[serde(default)]
    pub name: LenientString,
}

/// `opencollection.yml`: the collection root, stored unbundled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionRoot {
    #[serde(default = "spec_version")]
    pub opencollection: LenientString,
    #[serde(default)]
    pub info: CollectionInfo,
    #[serde(default, skip_serializing_if = "RequestDefaults::is_empty")]
    pub request: RequestDefaults,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundled: Option<bool>,
}

fn spec_version() -> LenientString {
    SPEC_VERSION.into()
}

impl CollectionRoot {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            opencollection: spec_version(),
            info: CollectionInfo { name: LenientString(name.into()) },
            request: RequestDefaults::default(),
            bundled: Some(false),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FolderType {
    Folder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FolderInfo {
    #[serde(default)]
    pub name: LenientString,
    #[serde(rename = "type", default = "folder_type")]
    pub kind: FolderType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<f64>,
}

fn folder_type() -> FolderType {
    FolderType::Folder
}

/// `<folder>/folder.yml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Folder {
    #[serde(default = "FolderInfo::unnamed")]
    pub info: FolderInfo,
    #[serde(default, skip_serializing_if = "RequestDefaults::is_empty")]
    pub request: RequestDefaults,
}

impl FolderInfo {
    fn unnamed() -> Self {
        Self { name: LenientString::default(), kind: FolderType::Folder, seq: None }
    }
}

impl Folder {
    pub fn named(name: impl Into<String>, seq: f64) -> Self {
        Self {
            info: FolderInfo { name: LenientString(name.into()), kind: FolderType::Folder, seq: Some(seq) },
            request: RequestDefaults::default(),
        }
    }
}

/// `environments/<name>.yml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Environment {
    #[serde(default)]
    pub name: LenientString,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Variable>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_values_resolve() {
        let parse = |yaml: &str| serde_yaml_ng::from_str::<Variable>(yaml).unwrap();
        assert_eq!(parse("name: a\nvalue: x").resolved_value(), "x");
        assert_eq!(parse("name: a\nvalue: 8080").resolved_value(), "8080");
        assert_eq!(parse("name: a\nvalue: {type: number, data: '2'}").resolved_value(), "2");
        let variants = "name: a\nvalue:\n  - {title: A, value: one}\n  - {title: B, value: two, selected: true}";
        assert_eq!(parse(variants).resolved_value(), "two");
        assert!(!parse("name: a\nsecret: true").is_active());
    }

    #[test]
    fn auth_shapes() {
        let parse = |yaml: &str| serde_yaml_ng::from_str::<Auth>(yaml).unwrap();
        assert_eq!(parse("inherit"), Auth::INHERIT);
        assert_eq!(parse("type: bearer\ntoken: t"), Auth::Mode(AuthMode::Bearer { token: "t".into() }));
        assert!(matches!(parse("type: oauth2\nflow: implicit"), Auth::Other(_)));
    }
}
