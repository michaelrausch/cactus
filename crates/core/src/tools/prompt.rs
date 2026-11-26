//! `- tool: prompt`: ask for values, which later steps of the run use.

use serde::{Deserialize, Serialize};

use super::{Answer, Ask, Tool, ToolCx, ToolOutput, title_or};
use crate::collection::LenientString;
use crate::env::Env;
use crate::{Error, Result};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Prompt {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub title: LenientString,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vars: Vec<PromptVar>,
}

/// One value to ask for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PromptVar {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub label: LenientString,
    /// Filled in, and used when no one is there to answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<LenientString>,
    /// Typed hidden and shown masked in results.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub secret: bool,
}

impl Tool for Prompt {
    fn title(&self) -> String {
        title_or(&self.title, "Prompt")
    }

    fn templates(&mut self) -> Vec<&mut String> {
        let mut texts = vec![&mut self.title.0];
        for var in &mut self.vars {
            texts.push(&mut var.label.0);
            texts.extend(var.default.as_mut().map(|default| &mut default.0));
        }
        texts
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        let vars: Vec<PromptVar> = self.vars.iter().filter(|var| !var.name.trim().is_empty()).cloned().collect();
        let answers = match cx.host.ask(Ask::Fields { title: self.title(), fields: vars.clone() }).await {
            // Possibly only some, e.g. from the CLI's `--var`.
            Answer::Given(answers) => answers,
            Answer::Stopped => return Err(Error::Tool("Stopped".into())),
            Answer::Unattended => Env::new(),
        };
        let unanswered: Vec<&str> = vars
            .iter()
            .filter(|var| var.default.is_none() && !answers.contains_key(var.name.trim()))
            .map(|var| var.name.trim())
            .collect();
        if !unanswered.is_empty() {
            return Err(Error::Tool(format!("Needs an answer for {}", unanswered.join(", "))));
        }
        for var in &vars {
            let name = var.name.trim();
            let value = answers
                .get(name)
                .cloned()
                .or_else(|| var.default.as_ref().map(|default| default.0.clone()))
                .unwrap_or_default();
            if var.secret {
                cx.set_secret(name, value);
            } else {
                cx.set(name, value);
            }
        }
        Ok(ToolOutput::default())
    }
}
