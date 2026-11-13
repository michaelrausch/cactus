//! `- tool: poll`: send a request again and again until its checks pass,
//! e.g. until a job is done. Captures are taken from the last response.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::wait::MAX_SECONDS;
use super::{Tool, ToolCx, ToolOutput};
use crate::collection::LenientString;
use crate::critter::FlowStep;
use crate::scenario::Assertion;
use crate::{Error, Result};

/// Seconds between tries, and to keep trying, when not set.
const DEFAULT_EVERY: u32 = 2;
const DEFAULT_TIMEOUT: u32 = 60;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Poll {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub request: LenientString,
    /// Variables for this request only, as a request step's `with`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub with: BTreeMap<String, LenientString>,
    /// What "done" is: checks on the response, as a request step's `assert`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub until: Vec<Assertion>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub capture: BTreeMap<String, LenientString>,
    /// Seconds between tries; 2 when not set.
    #[serde(skip_serializing_if = "is_zero")]
    pub every: u32,
    /// Seconds to keep trying; 60 when not set.
    #[serde(skip_serializing_if = "is_zero")]
    pub timeout: u32,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference.
fn is_zero(seconds: &u32) -> bool {
    *seconds == 0
}

const fn or(seconds: u32, default: u32) -> u32 {
    if seconds == 0 { default } else { seconds }
}

impl Tool for Poll {
    fn title(&self) -> String {
        format!("Poll {}", self.request.trim())
    }

    fn templates(&mut self) -> Vec<&mut String> {
        // `with` is filled in when the request is prepared, as for a request step.
        self.until.iter_mut().filter_map(|check| check.value.as_mut().map(|value| &mut value.0)).collect()
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        let (every, timeout) = (or(self.every, DEFAULT_EVERY), or(self.timeout, DEFAULT_TIMEOUT));
        if timeout > MAX_SECONDS {
            return Err(Error::Tool(format!("Poll for at most {MAX_SECONDS} seconds")));
        }
        let step = FlowStep {
            request: self.request.clone(),
            with: self.with.clone(),
            capture: self.capture.clone(),
            assert: self.until.clone(),
            ..FlowStep::default()
        };
        let mut waited = 0;
        for tries in 1.. {
            // A request that can't be sent may just not be up yet: try again.
            let why = match cx.runner.send(&step).await {
                Ok((response, true)) => {
                    let text = format!("Done after {tries} {}", if tries == 1 { "try" } else { "tries" });
                    return Ok(ToolOutput { status: Some(response.status), ..ToolOutput::text(text) });
                }
                Ok((response, false)) => match response.checks.iter().find(|check| !check.ok) {
                    Some(check) => format!("{}: {}", check.name, check.detail),
                    None => format!("status {}", response.status),
                },
                Err(error) => error.to_string(),
            };
            if waited + every > timeout {
                return Err(Error::Tool(format!("Not done after {tries} tries ({timeout}s): {why}")));
            }
            tokio::time::sleep(Duration::from_secs(u64::from(every))).await;
            waited += every;
        }
        unreachable!("the loop only ends by returning")
    }
}
