//! `- tool: webhook`: send an HTTP request that isn't one of the project's,
//! e.g. to tell another system the flow ran.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Tool, ToolCx, ToolOutput};
use crate::collection::LenientString;
use crate::{Error, Result};

/// How much of the response body a result keeps.
const BODY_PREVIEW_CHARS: usize = 500;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Webhook {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub url: LenientString,
    /// `POST` when not set.
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub method: LenientString,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, LenientString>,
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub body: LenientString,
}

impl Webhook {
    fn method(&self) -> String {
        let method = self.method.trim();
        if method.is_empty() { "POST".into() } else { method.to_uppercase() }
    }
}

impl Tool for Webhook {
    fn title(&self) -> String {
        format!("{} {}", self.method(), self.url.trim())
    }

    fn templates(&mut self) -> Vec<&mut String> {
        let mut texts = vec![&mut self.url.0, &mut self.body.0];
        texts.extend(self.headers.values_mut().map(|value| &mut value.0));
        texts
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        let method =
            reqwest::Method::from_bytes(self.method().as_bytes()).map_err(|_| Error::InvalidMethod(self.method()))?;
        let mut request = cx.client.request(method, url(&self.url)?);
        for (name, value) in &self.headers {
            request = request.header(name.trim(), value.trim());
        }
        if !self.body.is_empty() {
            let json = serde_json::from_str::<serde_json::Value>(&self.body).is_ok();
            if json && !self.headers.keys().any(|name| name.eq_ignore_ascii_case("content-type")) {
                request = request.header("content-type", "application/json");
            }
            request = request.body(self.body.0.clone());
        }
        finish(request.send().await?).await
    }
}

/// POST `body` as JSON to `url`: what most chat webhooks take.
pub async fn post_json(cx: &ToolCx<'_>, url: &str, body: &serde_json::Value) -> Result<ToolOutput> {
    let request = cx.client.post(self::url(url)?).header("content-type", "application/json");
    finish(request.body(serde_json::to_vec(body)?).send().await?).await
}

fn url(text: &str) -> Result<reqwest::Url> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Error::MissingUrl);
    }
    reqwest::Url::parse(text).map_err(|e| Error::InvalidUrl { url: text.into(), reason: e.to_string() })
}

/// The status and the start of the body; a status that isn't 2xx fails.
async fn finish(response: reqwest::Response) -> Result<ToolOutput> {
    let status = response.status();
    let body: String = response.text().await.unwrap_or_default().chars().take(BODY_PREVIEW_CHARS).collect();
    if !status.is_success() {
        let detail = if body.trim().is_empty() { String::new() } else { format!(": {}", body.trim()) };
        return Err(Error::Tool(format!("Got {}{detail}", status.as_u16())));
    }
    let text = (!body.trim().is_empty()).then(|| body.trim().to_string());
    Ok(ToolOutput { text, status: Some(status.as_u16()), ..ToolOutput::default() })
}
