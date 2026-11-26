//! Flow tools: steps that aren't requests, such as asking for a value,
//! showing text or posting to a webhook.
//!
//! # Adding a tool
//!
//! 1. Write `tools/<id>.rs` with a struct for its fields in the flow file
//!    and `impl Tool` for it. The struct's name, lowercased, is the `tool:`
//!    it's written as, so `Wait` is `- tool: wait`. Give it
//!    `#[serde(default)]`, and leave every empty field out when writing
//!    (`skip_serializing_if`: `""`, `[]`, `{}`, `false`, `0`), which is
//!    what the editor does too.
//! 2. Add `<id>::<Struct>` to [`tools!`] below.
//! 3. Add its editor to `ui/src/features/critters/flows/tools/` (the UI
//!    won't typecheck until you do).
//!
//! A tool's templated fields ([`Tool::templates`]) have `{{names}}` filled
//! in before it runs. It talks to whoever is running the flow through
//! [`ToolCx::host`], sends requests and runs flows as the run does through
//! [`ToolCx::runner`], and hands values to later steps with [`ToolCx::set`].

// Every tool's `run` is an `async fn`, awaiting or not, so they all look alike.
#![allow(clippy::unused_async_trait_impl)]

mod assert;
mod display;
mod findings;
mod notify;
mod open;
mod pick;
mod poll;
mod prompt;
mod random;
mod run;
mod set;
mod wait;

#[cfg(feature = "pro")]
mod cert;
#[cfg(feature = "pro")]
mod fuzz;
#[cfg(feature = "pro")]
mod headers;
#[cfg(feature = "pro")]
mod loadtest;
#[cfg(feature = "pro")]
mod mutate;
#[cfg(feature = "pro")]
mod ratelimit;
#[cfg(feature = "pro")]
mod slack;
#[cfg(feature = "pro")]
mod stats;
#[cfg(feature = "pro")]
mod webhook;

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

pub use self::findings::{
    Coverage, FailOn, Finding, FindingKey, Metric, Severity, StatusCount, findings_verdict, metrics_verdict,
};
pub use self::pick::CHOICE;
pub use self::prompt::PromptVar;
pub use self::random::GENERATORS;
use crate::Result;
#[cfg(feature = "pro")]
use crate::collection::LenientString;
use crate::critter::FlowStep;
use crate::env::Env;
use crate::http::Response;
use crate::runner::FlowRun;
use crate::scenario::Scenario;

/// One kind of flow step.
pub trait Tool {
    /// What the step is called in results and the run log.
    fn title(&self) -> String;

    /// Every field that may hold `{{name}}` templates. They are filled in
    /// with the step's variables before [`Tool::run`].
    fn templates(&mut self) -> Vec<&mut String> {
        Vec::new()
    }

    fn run(&self, cx: &mut ToolCx<'_>) -> impl Future<Output = Result<ToolOutput>> + Send;

    /// Why `output` fails the step, if it does. The output is still shown.
    fn verdict(&self, _output: &ToolOutput) -> Option<String> {
        None
    }
}

/// The registry: every tool, as `module::Struct`. Generates [`ToolStep`],
/// the flow file's `- tool: <id>` step, and its dispatch. `pro` tools are
/// only built with the `pro` feature.
macro_rules! tools {
    (
        free: [$($module:ident :: $tool:ident),* $(,)?],
        pro: [$($pro_module:ident :: $pro_tool:ident),* $(,)?] $(,)?
    ) => {
        /// A tool step as written in the flow file, tagged by `tool`.
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "tool", rename_all = "lowercase")]
        #[cfg_attr(feature = "specta", derive(specta::Type))]
        pub enum ToolStep {
            $($tool($module::$tool),)*
            $(#[cfg(feature = "pro")] $pro_tool($pro_module::$pro_tool),)*
        }

        pub use self::{$($module::$tool),*};
        #[cfg(feature = "pro")]
        pub use self::{$($pro_module::$pro_tool),*};

        impl ToolStep {
            /// Every tool's id in this build, as written after `tool:`.
            #[cfg(not(feature = "pro"))]
            pub const IDS: &[&str] = &[$(stringify!($module)),*];
            /// Every tool's id in this build, as written after `tool:`.
            #[cfg(feature = "pro")]
            pub const IDS: &[&str] = &[$(stringify!($module),)* $(stringify!($pro_module)),*];
            /// The Pro tools' ids, in every build: the free edition keeps
            /// their steps as written instead of failing to read the flow.
            pub const PRO_IDS: &[&str] = &[$(stringify!($pro_module)),*];

            pub fn id(&self) -> &'static str {
                match self {
                    $(Self::$tool(_) => stringify!($module),)*
                    $(#[cfg(feature = "pro")] Self::$pro_tool(_) => stringify!($pro_module),)*
                }
            }

            pub fn title(&self) -> String {
                match self {
                    $(Self::$tool(tool) => tool.title(),)*
                    $(#[cfg(feature = "pro")] Self::$pro_tool(tool) => tool.title(),)*
                }
            }

            pub fn templates(&mut self) -> Vec<&mut String> {
                match self {
                    $(Self::$tool(tool) => tool.templates(),)*
                    $(#[cfg(feature = "pro")] Self::$pro_tool(tool) => tool.templates(),)*
                }
            }

            pub async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
                match self {
                    $(Self::$tool(tool) => tool.run(cx).await,)*
                    $(#[cfg(feature = "pro")] Self::$pro_tool(tool) => tool.run(cx).await,)*
                }
            }

            pub fn verdict(&self, output: &ToolOutput) -> Option<String> {
                match self {
                    $(Self::$tool(tool) => tool.verdict(output),)*
                    $(#[cfg(feature = "pro")] Self::$pro_tool(tool) => tool.verdict(output),)*
                }
            }
        }
    };
}

tools! {
    free: [
        prompt::Prompt,
        display::Display,
        wait::Wait,
        open::Open,
        set::Set,
        notify::Notify,
        assert::Assert,
        poll::Poll,
        run::Run,
        pick::Pick,
    ],
    pro: [
        webhook::Webhook,
        slack::Slack,
        fuzz::Fuzz,
        headers::Headers,
        cert::Cert,
        ratelimit::Ratelimit,
        loadtest::Loadtest,
    ],
}

/// What a tool step did, for the step card, the run panel and the CLI.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ToolOutput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// An HTTP status, for tools that send something.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// The variables it set for later steps; secret ones as `••••`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, String>,
    /// What a security tool found, most severe first; `None` from a tool
    /// that doesn't scan, so an empty list means "found nothing".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub findings: Option<Vec<Finding>>,
    /// Numbers a tool measured, e.g. a load test's latencies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<Metric>,
    /// What it tried at each place, e.g. each value a fuzzer replaced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coverage: Vec<Coverage>,
}

impl ToolOutput {
    pub fn text(text: impl Into<String>) -> Self {
        Self { text: Some(text.into()), ..Self::default() }
    }

    /// What it found; empty when it didn't scan.
    pub fn found(&self) -> &[Finding] {
        self.findings.as_deref().unwrap_or_default()
    }
}

/// Shown in place of a secret value.
pub const MASK: &str = "••••";

/// A question for whoever runs the flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Ask {
    /// Fill in these values. With none, it's a Continue/Stop question.
    Fields { title: String, fields: Vec<PromptVar> },
    /// Read this, then carry on.
    Continue { title: String, text: String },
    /// Pick one of these; the answer is `{choice: "<index>"}`.
    Choose { title: String, choices: Vec<Choice> },
}

/// One option of [`Ask::Choose`]: what it shows, and the variables
/// choosing it sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Choice {
    pub label: String,
    pub values: BTreeMap<String, String>,
}

/// How a question was answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// The values, by name; empty for a Continue.
    Given(Env),
    /// They chose to stop the run.
    Stopped,
    /// Nobody to ask, e.g. a scheduled run or a CLI run without a terminal.
    Unattended,
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Whatever is running the flow: the desktop app, the CLI or the scheduler.
pub trait ToolHost: Send + Sync {
    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer>;
    /// Show a message without waiting for it to be read.
    fn show(&self, title: &str, text: &str);
    fn notify(&self, title: &str, body: &str);
    fn open_url(&self, url: &str) -> Result<()>;
    /// How far the long-running tool at step `index` has got. Called
    /// often; hosts throttle as they need.
    fn progress(&self, _index: u32, _done: u32, _total: u32) {}
}

/// No one is watching: questions go unanswered, notifications and URLs are
/// dropped. Scheduled runs and logging in again after a 401 use this.
pub struct Headless;

impl ToolHost for Headless {
    fn ask(&self, _ask: Ask) -> BoxFuture<'_, Answer> {
        Box::pin(async { Answer::Unattended })
    }

    fn show(&self, _title: &str, _text: &str) {}

    fn notify(&self, _title: &str, _body: &str) {}

    fn open_url(&self, _url: &str) -> Result<()> {
        Ok(())
    }
}

/// The run a tool is part of: sending requests and running flows as it does.
pub trait FlowRunner: Send + Sync {
    /// Send a request step as this run would (same critter, env and
    /// variables so far). Also says whether its checks passed.
    fn send<'a>(&'a self, step: &'a FlowStep) -> BoxFuture<'a, Result<(Response, bool)>>;

    /// Send a request step changed by `edit`, as this run would, keeping
    /// nothing: no history, no captured values, no logging in again on a
    /// 401. For fuzzers and load tests; safe to call concurrently.
    fn probe<'a>(
        &'a self,
        step: &'a FlowStep,
        edit: &'a (dyn Fn(&mut Scenario) + Sync),
    ) -> BoxFuture<'a, Result<Response>>;

    /// Run another flow, of `critter` or else the same critter, in this
    /// run's env. Its steps aren't reported one by one.
    fn run_flow<'a>(&'a self, critter: Option<&'a str>, flow: &'a str) -> BoxFuture<'a, Result<FlowRun>>;
}

/// What a running tool can reach.
pub struct ToolCx<'a> {
    pub client: &'a reqwest::Client,
    pub host: &'a dyn ToolHost,
    pub runner: &'a dyn FlowRunner,
    /// What the step sees: env, critter and everything set so far.
    vars: &'a Env,
    set: Env,
    secret: BTreeSet<String>,
    /// The step's place in its flow.
    index: u32,
}

impl<'a> ToolCx<'a> {
    pub fn new(client: &'a reqwest::Client, host: &'a dyn ToolHost, runner: &'a dyn FlowRunner, vars: &'a Env) -> Self {
        Self { client, host, runner, vars, set: Env::new(), secret: BTreeSet::new(), index: 0 }
    }

    /// The same, for step `index` of its flow.
    #[must_use]
    pub fn at(mut self, index: u32) -> Self {
        self.index = index;
        self
    }

    /// Report how far a long-running tool has got, e.g. requests sent.
    pub fn progress(&self, done: u32, total: u32) {
        self.host.progress(self.index, done, total);
    }

    /// Every variable the step can use, for tools that read values
    /// themselves rather than through [`Tool::templates`].
    pub fn vars(&self) -> &Env {
        self.vars
    }

    /// Give later steps of this run `{{name}}`. Never written to disk.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        self.secret.remove(&name);
        self.set.insert(name, value.into());
    }

    /// [`ToolCx::set`], shown as [`MASK`] in results.
    pub fn set_secret(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        self.set.insert(name.clone(), value.into());
        self.secret.insert(name);
    }

    /// The values set, and the same for display with secrets masked.
    pub fn finish(self) -> (Env, BTreeMap<String, String>) {
        let shown = self
            .set
            .iter()
            .map(|(name, value)| {
                let value = if self.secret.contains(name) { MASK.to_string() } else { value.clone() };
                (name.clone(), value)
            })
            .collect();
        (self.set, shown)
    }
}

/// A tool's `title` when set, or its default.
fn title_or(title: &str, default: &str) -> String {
    let title = title.trim();
    if title.is_empty() { default.to_string() } else { title.to_string() }
}

/// For `skip_serializing_if` on counts and seconds.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference.
fn is_zero(number: &u32) -> bool {
    *number == 0
}

/// The request step that tools sending one request many times probe with.
#[cfg(feature = "pro")]
fn request_step(request: &LenientString, with: &BTreeMap<String, LenientString>) -> FlowStep {
    FlowStep { request: request.clone(), with: with.clone(), ..FlowStep::default() }
}

/// A probe's edit that changes nothing.
#[cfg(feature = "pro")]
fn unchanged(_: &mut Scenario) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_tool_reads_as_its_id() {
        for id in ToolStep::IDS {
            let step: ToolStep = serde_json::from_value(serde_json::json!({ "tool": id })).unwrap_or_else(|e| {
                panic!("`tool: {id}` with no other fields should read with defaults: {e}");
            });
            assert_eq!(step.id(), *id);
            let written = serde_json::to_value(&step).unwrap();
            assert_eq!(written, serde_json::json!({ "tool": id }), "empty fields are left out");
        }
    }

    /// A run with nothing to send.
    struct NoRunner;

    impl FlowRunner for NoRunner {
        fn send<'a>(&'a self, _step: &'a FlowStep) -> BoxFuture<'a, Result<(Response, bool)>> {
            Box::pin(async { Err(crate::Error::Tool("no runner".into())) })
        }

        fn run_flow<'a>(&'a self, _critter: Option<&'a str>, _flow: &'a str) -> BoxFuture<'a, Result<FlowRun>> {
            Box::pin(async { Err(crate::Error::Tool("no runner".into())) })
        }

        fn probe<'a>(
            &'a self,
            _step: &'a FlowStep,
            _edit: &'a (dyn Fn(&mut Scenario) + Sync),
        ) -> BoxFuture<'a, Result<Response>> {
            Box::pin(async { Err(crate::Error::Tool("no runner".into())) })
        }
    }

    #[test]
    fn secrets_are_masked_in_what_is_shown() {
        let client = reqwest::Client::new();
        let vars = Env::new();
        let mut cx = ToolCx::new(&client, &Headless, &NoRunner, &vars);
        cx.set("a", "1");
        cx.set_secret("otp", "123456");
        let (set, shown) = cx.finish();
        assert_eq!(set["otp"], "123456");
        assert_eq!((shown["a"].as_str(), shown["otp"].as_str()), ("1", MASK));
    }
}
