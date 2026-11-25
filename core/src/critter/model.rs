//! A critter (a persona) and its flows, as stored in `critters/<id>/`.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::collection::{LenientString, Variable};
use crate::scenario::Assertion;
use crate::tools::ToolStep;
use crate::{Error, Result};

/// `critters/<id>/critter.yml`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Critter {
    #[serde(default)]
    pub name: LenientString,
    #[serde(default)]
    pub species: Species,
    #[serde(default, skip_serializing_if = "LenientString::is_empty")]
    pub notes: LenientString,
    /// The flow that logs this critter in again after a 401.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<LenientString>,
    /// Same shape as an environment's variables; `secret: true` values live
    /// in `.cactus/critters/<id>/secrets.json`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Variable>,
}

/// What the critter looks like. An unknown species reads as the default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Species {
    #[default]
    Roadrunner,
    Tortoise,
    Gecko,
    ElfOwl,
    Armadillo,
    Scorpion,
    Jackalope,
    Tumbleweed,
}

impl<'de> Deserialize<'de> for Species {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum Known {
            Roadrunner,
            Tortoise,
            Gecko,
            ElfOwl,
            Armadillo,
            Scorpion,
            Jackalope,
            Tumbleweed,
        }
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(match serde_json::from_value::<Known>(value) {
            Ok(Known::Roadrunner) | Err(_) => Self::Roadrunner,
            Ok(Known::Tortoise) => Self::Tortoise,
            Ok(Known::Gecko) => Self::Gecko,
            Ok(Known::ElfOwl) => Self::ElfOwl,
            Ok(Known::Armadillo) => Self::Armadillo,
            Ok(Known::Scorpion) => Self::Scorpion,
            Ok(Known::Jackalope) => Self::Jackalope,
            Ok(Known::Tumbleweed) => Self::Tumbleweed,
        })
    }
}

/// `critters/<id>/flows/<flow>.yml`: requests run in order as the critter.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Flow {
    #[serde(default)]
    pub name: LenientString,
    /// What runs the flow on its own, besides running it by hand.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<Trigger>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert: Option<AlertRule>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// One step of a flow: a request, or a tool (`- tool: prompt`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Step {
    Tool(ToolStep),
    Request(FlowStep),
}

impl Step {
    pub fn request(request: impl Into<LenientString>) -> Self {
        Self::Request(FlowStep { request: request.into(), ..FlowStep::default() })
    }

    pub fn as_request(&self) -> Option<&FlowStep> {
        match self {
            Self::Request(step) => Some(step),
            Self::Tool(_) => None,
        }
    }
}

/// A map with a `tool` key is a tool step; anything else is a request step,
/// so flows written before tools read unchanged.
impl<'de> Deserialize<'de> for Step {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let parsed = if value.get("tool").is_some() {
            serde_json::from_value(value).map(Self::Tool)
        } else {
            serde_json::from_value(value).map(Self::Request)
        };
        parsed.map_err(serde::de::Error::custom)
    }
}

impl Flow {
    /// The schedule the scheduler follows: the first one, if any.
    pub fn schedule(&self) -> Option<&Schedule> {
        self.triggers.iter().map(|Trigger::Schedule(schedule)| schedule).next()
    }
}

/// One request of a flow and what the flow adds to it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FlowStep {
    /// Project-relative request file. Checked when the step runs, so a bad
    /// path is a step error rather than a flow that won't load.
    #[serde(default)]
    pub request: LenientString,
    /// Variables for this step only, with the highest precedence.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub with: BTreeMap<String, LenientString>,
    /// `name: res.* expression`, saved to the critter's context.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub capture: BTreeMap<String, LenientString>,
    /// Replaces the request's own status assertions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect: Option<u16>,
    /// Extra checks, after the request's own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assert: Vec<Assertion>,
    /// Expressions whose values are compared with the previous run's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub watch: Vec<LenientString>,
}

/// Something that runs a flow on its own, named by `type` so other kinds
/// can join `schedule` later: `- {type: schedule, every: 5m}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Trigger {
    Schedule(Schedule),
}

/// Run every so often while cactus is open, on machines it's turned on for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Schedule {
    pub every: Interval,
    /// The env to run in; the active one when not set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
}

/// A schedule interval in seconds, written `30s`, `5m` or `1h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "specta", derive(specta::Type), specta(type = String))]
pub struct Interval(u32);

impl Interval {
    pub const MIN_SECONDS: u32 = 30;

    pub fn from_seconds(seconds: u32) -> Result<Self> {
        if seconds < Self::MIN_SECONDS {
            return Err(Error::InvalidInterval(format!("{seconds}s")));
        }
        Ok(Self(seconds))
    }

    pub fn parse(text: &str) -> Result<Self> {
        let invalid = || Error::InvalidInterval(text.to_string());
        let text = text.trim();
        let split = text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());
        let (digits, unit) = text.split_at(split);
        let amount: u32 = digits.parse().map_err(|_| invalid())?;
        let scale = match unit.trim() {
            "" | "s" => 1,
            "m" => 60,
            "h" => 3600,
            _ => return Err(invalid()),
        };
        let seconds = amount.checked_mul(scale).ok_or_else(invalid)?;
        Self::from_seconds(seconds).map_err(|_| invalid())
    }

    pub fn seconds(self) -> u32 {
        self.0
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            s if s % 3600 == 0 => write!(f, "{}h", s / 3600),
            s if s % 60 == 0 => write!(f, "{}m", s / 60),
            s => write!(f, "{s}s"),
        }
    }
}

impl Serialize for Interval {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Interval {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        // `every: 60` reads as seconds.
        let text = LenientString::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// Which changes of state alert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AlertRule {
    #[serde(default = "failure_only")]
    pub on: Vec<AlertOn>,
}

impl Default for AlertRule {
    fn default() -> Self {
        Self { on: failure_only() }
    }
}

fn failure_only() -> Vec<AlertOn> {
    vec![AlertOn::Failure]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum AlertOn {
    /// Starting to fail, and recovering.
    Failure,
    /// A watched value differing from the previous run.
    Change,
}

/// A captured value in `.cactus/critters/<id>/context.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContextEntry {
    pub value: String,
    /// The request that captured it, or `"edited"`.
    pub from: String,
    /// Unix epoch millis.
    // A JS `number`, accepting precision loss: epoch millis stay far below 2^53.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub at: u64,
}

/// Critter ids and flow ids become file names, so, like env names, they
/// are limited to `[A-Za-z0-9_-]+`.
fn is_valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn validate_critter_id(id: &str) -> Result<()> {
    if is_valid_id(id) { Ok(()) } else { Err(Error::InvalidCritterName(id.to_string())) }
}

pub fn validate_flow_id(id: &str) -> Result<()> {
    if is_valid_id(id) { Ok(()) } else { Err(Error::InvalidFlowName(id.to_string())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals() {
        let parse = |t: &str| Interval::parse(t).map(|i| (i.seconds(), i.to_string()));
        assert_eq!(parse("30s").unwrap(), (30, "30s".into()));
        assert_eq!(parse("5m").unwrap(), (300, "5m".into()));
        assert_eq!(parse("1h").unwrap(), (3600, "1h".into()));
        assert_eq!(parse("90").unwrap(), (90, "90s".into()));
        assert_eq!(parse("120s").unwrap(), (120, "2m".into()));
        for bad in ["29s", "0m", "", "m", "5d", "-5m", "99999999999h"] {
            assert!(matches!(Interval::parse(bad), Err(Error::InvalidInterval(_))), "{bad}");
        }
        let schedule: Schedule = serde_yaml_ng::from_str("every: 60").unwrap();
        assert_eq!(schedule.every.seconds(), 60);
        assert!(serde_yaml_ng::from_str::<Schedule>("every: 10s").is_err());
    }

    #[test]
    fn reads_the_documented_flow() {
        let yaml = r"
name: Staging health
triggers:
  - type: schedule
    every: 5m
    env: staging
alert:
  on: [failure, change]
steps:
  - request: auth/login.yml
    with:
      username: bob@example.com
      count: 5
    capture:
      access_token: res.body.access_token
  - request: users/delete.yml
    expect: 403
  - request: status.yml
    assert:
      - expression: res.body.db
        operator: eq
        value: ok
    watch:
      - res.body.version
  - tool: wait
    seconds: 5
";
        let flow: Flow = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(flow.schedule().unwrap().env.as_deref(), Some("staging"));
        let written = serde_yaml_ng::to_string(&flow.triggers).unwrap();
        assert_eq!(written, "- type: schedule\n  every: 5m\n  env: staging\n");
        assert_eq!(flow.alert.unwrap().on, [AlertOn::Failure, AlertOn::Change]);
        let request = |i: usize| flow.steps[i].as_request().unwrap();
        assert_eq!(request(0).with["count"], "5");
        assert_eq!(request(1).expect, Some(403));
        assert_eq!(request(2).watch, [LenientString::from("res.body.version")]);
        assert!(matches!(&flow.steps[3], Step::Tool(tool) if tool.id() == "wait"));
        let steps = serde_yaml_ng::to_string(&flow.steps[3]).unwrap();
        assert_eq!(steps, "tool: wait\nseconds: 5\n");
        let default_alert: AlertRule = serde_yaml_ng::from_str("{}").unwrap();
        assert_eq!(default_alert.on, [AlertOn::Failure]);
    }

    #[test]
    fn an_unknown_tool_is_an_error_naming_it() {
        let error = serde_yaml_ng::from_str::<Flow>("steps:\n  - tool: teleport\n").unwrap_err().to_string();
        assert!(error.contains("teleport"), "{error}");
    }

    #[test]
    fn unknown_species_reads_as_default() {
        let critter: Critter = serde_yaml_ng::from_str("name: 42\nspecies: dragon").unwrap();
        assert_eq!((critter.name.as_str(), critter.species), ("42", Species::Roadrunner));
        let owl: Critter = serde_yaml_ng::from_str("species: elf-owl").unwrap();
        assert_eq!(owl.species, Species::ElfOwl);
    }

    #[test]
    fn ids() {
        assert!(validate_critter_id("bob_2-x").is_ok());
        for bad in ["", "../x", "a/b", "a.b", "a b"] {
            assert!(validate_critter_id(bad).is_err() && validate_flow_id(bad).is_err(), "{bad}");
        }
    }
}
