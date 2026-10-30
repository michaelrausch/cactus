use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

/// Bodies larger than this are truncated before going to the UI.
const MAX_LIVE_BODY_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Response {
    /// The request as actually sent, after env substitution.
    pub method: String,
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub truncated: bool,
    // Exposed to TypeScript as `number`: the values stay far below 2^53.
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub time_ms: u64,
    // Exposed to TypeScript as `number`: the values stay far below 2^53.
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub size_bytes: usize,
    pub unresolved: Vec<String>,
    /// What cactus left out, e.g. scripts it can't run.
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub captured: BTreeMap<String, String>,
    #[serde(default)]
    pub checks: Vec<Check>,
}

/// The outcome of one assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// What we know about the request once it has been sent.
pub(crate) struct SentRequest {
    pub method: String,
    pub url: String,
    pub unresolved: Vec<String>,
    pub warnings: Vec<String>,
}

/// What came back over the wire.
pub(crate) struct ReceivedResponse<'a> {
    pub status: StatusCode,
    pub headers: &'a HeaderMap,
    pub body: &'a [u8],
    pub elapsed: Duration,
}

impl Response {
    pub(crate) fn new(sent: SentRequest, received: &ReceivedResponse) -> Self {
        let body = received.body;
        Self {
            method: sent.method,
            url: sent.url,
            status: received.status.as_u16(),
            status_text: received.status.canonical_reason().unwrap_or_default().to_string(),
            headers: header_pairs(received.headers),
            body: String::from_utf8_lossy(&body[..body.len().min(MAX_LIVE_BODY_BYTES)]).into_owned(),
            truncated: body.len() > MAX_LIVE_BODY_BYTES,
            time_ms: u64::try_from(received.elapsed.as_millis()).unwrap_or(u64::MAX),
            size_bytes: body.len(),
            unresolved: sent.unresolved,
            warnings: sent.warnings,
            captured: BTreeMap::new(),
            checks: Vec::new(),
        }
    }

    /// First header with this name, case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value.as_str())
    }
}

fn header_pairs(headers: &HeaderMap) -> Vec<(String, String)> {
    headers.iter().map(|(k, v)| (k.to_string(), String::from_utf8_lossy(v.as_bytes()).into_owned())).collect()
}
