//! `- tool: wait`: pause, e.g. for an eventually consistent API.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{Tool, ToolCx, ToolOutput};
use crate::{Error, Result};

/// The longest wait, so a typo can't stall a schedule for hours.
pub const MAX_SECONDS: u32 = 600;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Wait {
    #[serde(skip_serializing_if = "is_zero")]
    pub seconds: u32,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference.
fn is_zero(seconds: &u32) -> bool {
    *seconds == 0
}

impl Tool for Wait {
    fn title(&self) -> String {
        format!("Wait {}s", self.seconds)
    }

    async fn run(&self, _cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        if self.seconds > MAX_SECONDS {
            return Err(Error::Tool(format!("Wait at most {MAX_SECONDS} seconds")));
        }
        tokio::time::sleep(Duration::from_secs(u64::from(self.seconds))).await;
        Ok(ToolOutput::default())
    }
}
