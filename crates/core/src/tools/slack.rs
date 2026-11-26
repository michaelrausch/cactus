//! `- tool: slack`: post a message to a Slack incoming webhook.

use serde::{Deserialize, Serialize};

use super::webhook::post_json;
use super::{Tool, ToolCx, ToolOutput};
use crate::Result;
use crate::collection::LenientString;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Slack {
    /// The incoming webhook URL; best kept in a secret, `{{SLACK_WEBHOOK}}`.
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub webhook: LenientString,
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub text: LenientString,
}

impl Tool for Slack {
    fn title(&self) -> String {
        "Slack".into()
    }

    fn templates(&mut self) -> Vec<&mut String> {
        vec![&mut self.webhook.0, &mut self.text.0]
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        post_json(cx, &self.webhook, &serde_json::json!({ "text": self.text.0 })).await?;
        Ok(ToolOutput::text(self.text.0.clone()))
    }
}
