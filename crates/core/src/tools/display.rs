//! `- tool: display`: show a titled message; with `pause: true` the run
//! waits for Continue.

use serde::{Deserialize, Serialize};

use super::{Answer, Ask, Tool, ToolCx, ToolOutput, title_or};
use crate::collection::LenientString;
use crate::{Error, Result};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Display {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub title: LenientString,
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub text: LenientString,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub pause: bool,
}

impl Tool for Display {
    fn title(&self) -> String {
        title_or(&self.title, "Display")
    }

    fn templates(&mut self) -> Vec<&mut String> {
        vec![&mut self.title.0, &mut self.text.0]
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        let (title, text) = (self.title(), self.text.0.clone());
        if self.pause {
            if cx.host.ask(Ask::Continue { title, text: text.clone() }).await == Answer::Stopped {
                return Err(Error::Tool("Stopped".into()));
            }
        } else {
            cx.host.show(&title, &text);
        }
        Ok(ToolOutput::text(text))
    }
}
