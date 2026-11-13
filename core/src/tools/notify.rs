//! `- tool: notify`: a desktop notification.

use serde::{Deserialize, Serialize};

use super::{Tool, ToolCx, ToolOutput, title_or};
use crate::Result;
use crate::collection::LenientString;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Notify {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub title: LenientString,
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub body: LenientString,
}

impl Tool for Notify {
    fn title(&self) -> String {
        "Notify".into()
    }

    fn templates(&mut self) -> Vec<&mut String> {
        vec![&mut self.title.0, &mut self.body.0]
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        let title = title_or(&self.title, "cactus");
        cx.host.notify(&title, self.body.trim());
        Ok(ToolOutput::text(title))
    }
}
