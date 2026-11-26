//! `- tool: set`: set variables for later steps, from templates or
//! generators such as `$uuid`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Tool, ToolCx, ToolOutput, random};
use crate::Result;
use crate::collection::LenientString;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Set {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, LenientString>,
}

impl Tool for Set {
    fn title(&self) -> String {
        "Set".into()
    }

    fn templates(&mut self) -> Vec<&mut String> {
        self.values.values_mut().map(|value| &mut value.0).collect()
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        for (name, value) in &self.values {
            let name = name.trim();
            if !name.is_empty() {
                cx.set(name, random::generate(value.trim()).unwrap_or_else(|| value.0.clone()));
            }
        }
        Ok(ToolOutput::default())
    }
}
