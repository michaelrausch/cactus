//! TypeScript-facing stand-ins for types specta can't export directly.
//!
//! `serde_json::Value` can't be exported because its numbers may be 64-bit.
//! In the UI every JSON number is a JS `number` anyway, so `Value` fields are
//! described with [`JsonValue`] instead. This type only describes the shape;
//! it is never constructed.

use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Serialize, specta::Type)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum JsonValue {
    Null(()),
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}
