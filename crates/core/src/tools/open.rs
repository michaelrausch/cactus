//! `- tool: open`: open a URL in the browser, e.g. an OAuth sign-in page.

use serde::{Deserialize, Serialize};

use super::{Tool, ToolCx, ToolOutput};
use crate::collection::LenientString;
use crate::{Error, Result};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Open {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub url: LenientString,
}

impl Tool for Open {
    fn title(&self) -> String {
        "Open".into()
    }

    fn templates(&mut self) -> Vec<&mut String> {
        vec![&mut self.url.0]
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        let url = self.url.trim();
        // Only web pages: a flow file from someone else mustn't open apps or files.
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err(Error::InvalidUrl { url: url.into(), reason: "open only takes http(s) URLs".into() });
        }
        cx.host.open_url(url)?;
        Ok(ToolOutput::text(url))
    }
}
