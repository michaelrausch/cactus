//! `- tool: run`: run another flow, this critter's or another's, as one
//! step, e.g. setup that several flows share.

use serde::{Deserialize, Serialize};

use super::{Tool, ToolCx, ToolOutput};
use crate::collection::LenientString;
use crate::{Error, Result};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Run {
    /// The flow's id, its file name without `.yml`.
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub flow: LenientString,
    /// Whose flow, run as them; this flow's critter when not set.
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub critter: LenientString,
}

impl Tool for Run {
    fn title(&self) -> String {
        match self.critter.trim() {
            "" => format!("Run {}", self.flow.trim()),
            critter => format!("Run {critter}/{}", self.flow.trim()),
        }
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        let flow = self.flow.trim();
        if flow.is_empty() {
            return Err(Error::Tool("No flow to run".into()));
        }
        let critter = Some(self.critter.trim()).filter(|critter| !critter.is_empty());
        let run = cx.runner.run_flow(critter, flow).await?;
        if let Some(failure) = run.failure() {
            return Err(Error::Tool(failure));
        }
        let passed = run.steps.iter().filter(|step| step.ok).count();
        Ok(ToolOutput::text(format!("{passed}/{} passed", run.steps.len())))
    }
}
