//! `- tool: fuzz`: send a request again with each of its values replaced,
//! one at a time: by payloads that break careless code (SQL, HTML, paths,
//! shell commands, huge and mistyped values) and by inputs mutated from the
//! value itself. Reports responses that show a problem (database errors,
//! stack traces, reflected markup, file contents, no answer at all), each
//! with a curl command to send it again, and what every value got back.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::{LazyLock, Mutex};

use futures_util::stream::{self, StreamExt};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::findings::{Exports, export};
use super::mutate::{Rng, mutations, random_seed};
use super::{
    Coverage, FailOn, Finding, MASK, Metric, Severity, StatusCount, Tool, ToolCx, ToolOutput, findings_verdict,
    is_zero, request_step,
};
use crate::collection::LenientString;
use crate::http::Response;
use crate::scenario::{Body, ParamKind, Scenario, TypedBody};
use crate::{Error, Result};

/// Requests in flight at once when not set, and the most.
const DEFAULT_CONCURRENCY: u32 = 4;
pub const MAX_CONCURRENCY: u32 = 32;
/// The most requests one step sends.
pub const MAX_REQUESTS: usize = 2000;
/// Mutated inputs per value when not set, and the most.
const DEFAULT_ITERATIONS: u32 = 20;
pub const MAX_ITERATIONS: u32 = 200;
/// Headers, query pairs and body fields whose values a repro command masks,
/// by name or part of one.
const SECRET_NAMES: &[&str] = &["authorization", "cookie", "token", "key", "secret", "session", "password"];
/// Seconds the sleep payloads ask for, and how much slower than the
/// baseline counts as having slept.
const SLEEP_SECONDS: u64 = 6;
const SLOWER_MS: u64 = 5000;
/// Stands in a JSON body for a payload whose text goes in as raw JSON.
const RAW_JSON_MARK: &str = "cactus:raw-json";
/// How much of a response a finding quotes.
const SNIPPET_CHARS: usize = 200;
/// Headers left alone, along with every `Content-*`: changing them tests
/// the transport or auth, not the API.
const KEPT_HEADERS: &[&str] = &["authorization", "cookie", "host", "transfer-encoding", "connection"];

/// Which values of the request to replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum FuzzTarget {
    Query,
    Path,
    Headers,
    Body,
}

/// A family of payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum PayloadKind {
    Sqli,
    Xss,
    Traversal,
    Command,
    Overflow,
    Types,
    /// Inputs made from each value's own: see [`super::mutate`].
    Mutate,
}

impl PayloadKind {
    const ALL: [Self; 7] =
        [Self::Sqli, Self::Xss, Self::Traversal, Self::Command, Self::Overflow, Self::Types, Self::Mutate];
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Fuzz {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub request: LenientString,
    /// Variables for this request only, as a request step's `with`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub with: BTreeMap<String, LenientString>,
    /// What to replace; query, path and body when not set.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<FuzzTarget>,
    /// Which payloads to send; all when not set.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub payloads: Vec<PayloadKind>,
    /// Mutated inputs per value, with `mutate`; 20 when not set.
    #[serde(skip_serializing_if = "is_zero")]
    pub iterations: u32,
    /// Makes the same mutations again; a random one when not set, shown in
    /// the step's output.
    #[serde(skip_serializing_if = "is_zero")]
    pub seed: u32,
    /// How many to have in flight at once; 4 when not set.
    #[serde(skip_serializing_if = "is_zero")]
    pub concurrency: u32,
    #[serde(skip_serializing_if = "FailOn::is_default")]
    pub fail_on: FailOn,
    /// Findings or metrics for later steps: `issues: findings`, `highs: findings.high`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub set: BTreeMap<String, LenientString>,
}

/// What `set` can name.
const EXPORTS: Exports = Exports { metrics: &["requests", "errors"], findings: true };

impl Fuzz {
    fn targets(&self) -> BTreeSet<FuzzTarget> {
        if self.targets.is_empty() {
            BTreeSet::from([FuzzTarget::Query, FuzzTarget::Path, FuzzTarget::Body])
        } else {
            self.targets.iter().copied().collect()
        }
    }

    fn kinds(&self) -> BTreeSet<PayloadKind> {
        if self.payloads.is_empty() { PayloadKind::ALL.into() } else { self.payloads.iter().copied().collect() }
    }

    /// Every input to send: for each slot, the fixed payloads it can take,
    /// then its mutations.
    fn cases(&self, slots: &[Slot], seed: u32) -> Vec<(usize, Payload)> {
        let kinds = self.kinds();
        let fixed = payloads(&kinds);
        let iterations = if self.iterations == 0 { DEFAULT_ITERATIONS } else { self.iterations };
        let mut rng = Rng::new(seed);
        let mut cases = Vec::new();
        for (s, slot) in slots.iter().enumerate() {
            cases.extend(fixed.iter().filter(|payload| slot.takes(payload)).map(|payload| (s, payload.clone())));
            if kinds.contains(&PayloadKind::Mutate) {
                let made = mutations(&slot.original, iterations as usize, &mut rng);
                cases.extend(made.into_iter().map(Payload::mutated).filter(|p| slot.takes(p)).map(|p| (s, p)));
            }
        }
        cases
    }
}

/// What one slot got back.
#[derive(Debug, Clone, Default)]
struct Tally {
    statuses: BTreeMap<u16, u32>,
    findings: u32,
}

impl Tally {
    fn coverage(self, location: &str) -> Coverage {
        let mut statuses: Vec<StatusCount> =
            self.statuses.into_iter().map(|(status, count)| StatusCount { status, count }).collect();
        statuses.sort_by_key(|status| std::cmp::Reverse(status.count));
        Coverage {
            location: location.into(),
            requests: statuses.iter().map(|status| status.count).sum(),
            statuses,
            findings: self.findings,
        }
    }
}

impl Tool for Fuzz {
    fn title(&self) -> String {
        format!("Fuzz {}", self.request.trim())
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        EXPORTS.check(&self.set)?;
        if self.concurrency > MAX_CONCURRENCY {
            return Err(Error::Tool(format!("Send at most {MAX_CONCURRENCY} at once")));
        }
        if self.iterations > MAX_ITERATIONS {
            return Err(Error::Tool(format!("At most {MAX_ITERATIONS} iterations")));
        }
        let step = request_step(&self.request, &self.with);
        // The baseline, which also shows what the request looks like.
        let prepared = Mutex::new(None);
        let keep = |scenario: &mut Scenario| *prepared.lock().unwrap() = Some(scenario.clone());
        let baseline = cx.runner.probe(&step, &keep).await?;
        let scenario = prepared.into_inner().unwrap().unwrap_or_default();

        let slots = slots(&scenario, &self.targets());
        let seed = if self.seed == 0 { random_seed() } else { self.seed };
        let cases = self.cases(&slots, seed);
        if cases.is_empty() {
            return Err(Error::Tool("Nothing to fuzz in this request".into()));
        }
        if cases.len() > MAX_REQUESTS {
            return Err(Error::Tool(format!(
                "{} requests; at most {MAX_REQUESTS}: choose fewer targets, payloads or iterations",
                cases.len()
            )));
        }

        let total = u32::try_from(cases.len()).unwrap_or(u32::MAX);
        let concurrency = if self.concurrency == 0 { DEFAULT_CONCURRENCY } else { self.concurrency };
        // By index: futures borrowing them by reference trip up `Send` inference.
        let (step, all_slots, all_cases, shared) = (&step, &slots, &cases, &*cx);
        let mut sends = stream::iter(0..cases.len())
            .map(|i| async move {
                let (s, payload) = &all_cases[i];
                let edit = |scenario: &mut Scenario| all_slots[*s].apply(scenario, payload);
                (i, shared.runner.probe(step, &edit).await)
            })
            .buffer_unordered(concurrency as usize);
        let mut tally: Vec<Tally> = vec![Tally::default(); slots.len()];
        let (mut findings, mut found, mut sent, mut errors) = (Vec::new(), BTreeSet::new(), 0, 0);
        while let Some((i, outcome)) = sends.next().await {
            let (s, payload) = &cases[i];
            let slot = &slots[*s];
            sent += 1;
            shared.progress(sent, total);
            let (status, problems) = match outcome {
                Ok(response) => {
                    let mut problems = detect(&slot.location, payload, &response, &baseline);
                    for problem in &mut problems {
                        problem.repro = curl(&response);
                    }
                    (response.status, problems)
                }
                Err(error) => (0, vec![no_response(&slot.location, payload, &error.to_string())]),
            };
            if status == 0 || status >= 500 {
                errors += 1;
            }
            *tally[*s].statuses.entry(status).or_default() += 1;
            for problem in problems {
                // One finding per rule and place, whichever input found it first.
                if found.insert((problem.rule.clone(), problem.location.clone())) {
                    tally[*s].findings += 1;
                    findings.push(problem);
                }
            }
        }
        drop(sends);
        let coverage = slots.iter().zip(tally).map(|(slot, tally)| tally.coverage(&slot.location)).collect();
        let text = format!("{} values, {sent} requests, seed {seed}", slots.len());
        let metrics = vec![
            Metric::new("requests", "Requests", f64::from(sent), ""),
            Metric::new("errors", "Errors", f64::from(errors), ""),
        ];
        let output = ToolOutput { findings: Some(findings), metrics, coverage, ..ToolOutput::text(text) };
        export(cx, &self.set, &output)?;
        Ok(output)
    }

    fn verdict(&self, output: &ToolOutput) -> Option<String> {
        findings_verdict(self.fail_on, output.findings.as_deref())
    }
}

/// One value to send instead of a request's own.
#[derive(Debug, Clone, PartialEq)]
struct Payload {
    kind: PayloadKind,
    text: String,
    /// As a JSON body's value: the text as a string, or a value of another type.
    json: Value,
    /// Asks the server to sleep for [`SLEEP_SECONDS`].
    sleeps: bool,
}

impl Payload {
    fn text(kind: PayloadKind, text: impl Into<String>) -> Self {
        let text = text.into();
        Self { kind, json: Value::String(text.clone()), text, sleeps: false }
    }

    fn sleeping(kind: PayloadKind, text: &str) -> Self {
        Self { sleeps: true, ..Self::text(kind, text.replace("SECONDS", &SLEEP_SECONDS.to_string())) }
    }

    fn typed(text: &str, json: Value) -> Self {
        Self { json, ..Self::text(PayloadKind::Types, text) }
    }

    /// A mutated value: a string as itself, anything else as its JSON.
    fn mutated(json: Value) -> Self {
        let text = match &json {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        Self { kind: PayloadKind::Mutate, text, json, sleeps: false }
    }

    /// Whether it can be a header value.
    fn fits_header(&self) -> bool {
        self.text.bytes().all(|byte| byte == b'\t' || (0x20..0x7f).contains(&byte))
    }
}

/// The payloads of `kinds`.
fn payloads(kinds: &BTreeSet<PayloadKind>) -> Vec<Payload> {
    use PayloadKind::{Command, Overflow, Sqli, Traversal, Xss};
    let mut all = Vec::new();
    for kind in kinds {
        match kind {
            Sqli => {
                all.extend(
                    ["'", "\"", "' OR '1'='1", "1' ORDER BY 999-- -", "1 UNION SELECT NULL-- -"]
                        .map(|text| Payload::text(Sqli, text)),
                );
                all.extend(
                    ["' OR SLEEP(SECONDS)-- -", "'; WAITFOR DELAY '0:0:SECONDS'--", "1) OR pg_sleep(SECONDS)--"]
                        .map(|text| Payload::sleeping(Sqli, text)),
                );
            }
            Xss => all.extend(
                [
                    "<script>alert(1337)</script>",
                    "\"><img src=x onerror=alert(1337)>",
                    "<svg/onload=alert(1337)>",
                    "'\"><cactus-xss>",
                ]
                .map(|text| Payload::text(Xss, text)),
            ),
            Traversal => all.extend(
                [
                    "../../../../../../../../etc/passwd",
                    "....//....//....//....//....//etc/passwd",
                    "..%2f..%2f..%2f..%2f..%2f..%2fetc%2fpasswd",
                    "/etc/passwd",
                    "..\\..\\..\\..\\..\\..\\windows\\win.ini",
                    "file:///etc/passwd",
                ]
                .map(|text| Payload::text(Traversal, text)),
            ),
            Command => {
                all.extend([";id", "|id", "`id`", "$(id)", "&& id"].map(|text| Payload::text(Command, text)));
                all.extend([";sleep SECONDS", "|sleep SECONDS"].map(|text| Payload::sleeping(Command, text)));
            }
            Overflow => all.extend(
                [
                    "A".repeat(10_000),
                    "%s%s%s%s%n".into(),
                    "%x%x%x%x".into(),
                    "\0".into(),
                    "\u{feff}\u{202e}cactus".into(),
                ]
                .map(|text| Payload::text(Overflow, text)),
            ),
            PayloadKind::Types => all.extend([
                Payload::typed("", Value::String(String::new())),
                Payload::typed("null", Value::Null),
                Payload::typed("true", Value::Bool(true)),
                Payload::typed("-1", Value::from(-1)),
                // 2^64: one more than a u64 holds.
                Payload::typed("18446744073709551616", Value::String(RAW_JSON_MARK.into())),
                Payload::typed("[]", Value::Array(Vec::new())),
                Payload::typed("{}", Value::Object(serde_json::Map::new())),
            ]),
            // Made per value, from its own: see `Fuzz::cases`.
            PayloadKind::Mutate => {}
        }
    }
    all
}

/// Where a value sits in the request.
#[derive(Debug, Clone, PartialEq)]
enum Place {
    /// The `n`th `name=value` pair of the URL's query.
    Query(usize),
    /// An index into `http.params`.
    Path(usize),
    /// An index into `http.headers`.
    Header(usize),
    /// A leaf of a JSON body.
    Json(Vec<Key>),
    /// An index into a form body's fields.
    Form(usize),
}

#[derive(Debug, Clone, PartialEq)]
enum Key {
    Field(String),
    Index(usize),
}

#[derive(Debug, Clone, PartialEq)]
struct Slot {
    place: Place,
    /// How findings name it: `query.id`, `header.X-Tenant`, `body.user.name`.
    location: String,
    /// What it holds in the request, which mutations start from.
    original: Value,
}

impl Slot {
    fn takes(&self, payload: &Payload) -> bool {
        !matches!(self.place, Place::Header(_)) || payload.fits_header()
    }

    /// Put `payload` in this slot of `scenario`.
    fn apply(&self, scenario: &mut Scenario, payload: &Payload) {
        let http = &mut scenario.http;
        match &self.place {
            Place::Query(n) => http.url.0 = replace_query_value(&http.url, *n, &encode(&payload.text)),
            Place::Path(i) => http.params[*i].value.0 = encode(&payload.text),
            Place::Header(i) => http.headers[*i].value.0.clone_from(&payload.text),
            Place::Json(keys) => {
                if let Some(Body::Typed(TypedBody::Json { data })) = &mut http.body
                    && let Ok(mut json) = serde_json::from_str::<Value>(data)
                    && let Some(leaf) = leaf_mut(&mut json, keys)
                {
                    *leaf = payload.json.clone();
                    data.0 = json.to_string();
                    if payload.json == RAW_JSON_MARK {
                        // A number serde_json can't hold goes in as text.
                        data.0 = data.0.replacen(&format!("\"{RAW_JSON_MARK}\""), &payload.text, 1);
                    }
                }
            }
            Place::Form(i) => {
                if let Some(Body::Typed(TypedBody::FormUrlencoded { data })) = &mut http.body {
                    data[*i].value.0.clone_from(&payload.text);
                }
            }
        }
    }
}

/// Every value of `scenario` in `targets`.
fn slots(scenario: &Scenario, targets: &BTreeSet<FuzzTarget>) -> Vec<Slot> {
    let http = &scenario.http;
    let mut slots = Vec::new();
    let mut add = |place, location: String, original: Value| slots.push(Slot { place, location, original });
    let text = |text: &str| Value::String(text.to_string());
    if targets.contains(&FuzzTarget::Query) {
        for (n, (name, value)) in query_pairs(&http.url).into_iter().enumerate() {
            add(Place::Query(n), format!("query.{name}"), text(value));
        }
    }
    if targets.contains(&FuzzTarget::Path) {
        for (i, param) in http.params.iter().enumerate() {
            if param.kind == ParamKind::Path && !param.disabled && !param.name.is_empty() {
                add(Place::Path(i), format!("path.{}", param.name.trim()), text(&param.value));
            }
        }
    }
    if targets.contains(&FuzzTarget::Headers) {
        for (i, header) in http.headers.iter().enumerate() {
            let name = header.name.trim();
            let lower = name.to_ascii_lowercase();
            let kept = KEPT_HEADERS.contains(&lower.as_str()) || lower.starts_with("content-");
            if !header.disabled && !name.is_empty() && !kept {
                add(Place::Header(i), format!("header.{name}"), text(&header.value));
            }
        }
    }
    if targets.contains(&FuzzTarget::Body) {
        match &http.body {
            Some(Body::Typed(TypedBody::Json { data })) => {
                // A body that's only JSON once its `{{variables}}` are filled in is left alone.
                if let Ok(json) = serde_json::from_str::<Value>(data) {
                    let mut leaves = Vec::new();
                    collect_leaves(&json, &mut Vec::new(), &mut leaves);
                    for (keys, value) in leaves {
                        let location = format!("body{}", json_path(&keys));
                        add(Place::Json(keys), location, value);
                    }
                }
            }
            Some(Body::Typed(TypedBody::FormUrlencoded { data })) => {
                for (i, field) in data.iter().enumerate() {
                    if !field.disabled && !field.name.is_empty() {
                        add(Place::Form(i), format!("form.{}", field.name.trim()), text(&field.value));
                    }
                }
            }
            _ => {}
        }
    }
    number_repeats(&mut slots);
    slots
}

/// Tell apart values sharing a name, like `?id=1&id=2`, as `query.id[0]`
/// and `query.id[1]`: findings and coverage are kept per location.
fn number_repeats(slots: &mut [Slot]) {
    let mut counts = BTreeMap::<String, usize>::new();
    for slot in slots.iter() {
        *counts.entry(slot.location.clone()).or_default() += 1;
    }
    let mut seen = BTreeMap::<String, usize>::new();
    for slot in slots.iter_mut().filter(|slot| counts[&slot.location] > 1) {
        let n = seen.entry(slot.location.clone()).or_default();
        let _ = write!(slot.location, "[{n}]");
        *n += 1;
    }
}

/// The `name=value` pairs in `url`'s query, in order.
fn query_pairs(url: &str) -> Vec<(&str, &str)> {
    let Some((_, query)) = url.split_once('?') else { return Vec::new() };
    let query = query.split('#').next().unwrap_or_default();
    query.split('&').filter(|pair| !pair.is_empty()).map(|pair| pair.split_once('=').unwrap_or((pair, ""))).collect()
}

/// `url` with the value of its `n`th query pair replaced by `value`.
fn replace_query_value(url: &str, n: usize, value: &str) -> String {
    let Some((base, rest)) = url.split_once('?') else { return url.to_string() };
    let (query, fragment) = rest.split_once('#').map_or((rest, None), |(query, fragment)| (query, Some(fragment)));
    let pairs: Vec<String> = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .enumerate()
        .map(|(i, pair)| {
            if i == n {
                let name = pair.split('=').next().unwrap_or_default();
                format!("{name}={value}")
            } else {
                pair.to_string()
            }
        })
        .collect();
    let fragment = fragment.map_or_default(|fragment| format!("#{fragment}"));
    format!("{base}?{}{fragment}", pairs.join("&"))
}

/// Percent-encode all but unreserved characters, for a URL.
fn encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// Every leaf of `json` (a value that isn't an object or array), by its keys.
fn collect_leaves(json: &Value, keys: &mut Vec<Key>, leaves: &mut Vec<(Vec<Key>, Value)>) {
    match json {
        Value::Object(map) => {
            for (name, value) in map {
                keys.push(Key::Field(name.clone()));
                collect_leaves(value, keys, leaves);
                keys.pop();
            }
        }
        Value::Array(items) => {
            for (i, value) in items.iter().enumerate() {
                keys.push(Key::Index(i));
                collect_leaves(value, keys, leaves);
                keys.pop();
            }
        }
        _ if keys.is_empty() => {}
        leaf => leaves.push((keys.clone(), leaf.clone())),
    }
}

fn leaf_mut<'a>(json: &'a mut Value, keys: &[Key]) -> Option<&'a mut Value> {
    keys.iter().try_fold(json, |value, key| match key {
        Key::Field(name) => value.get_mut(name.as_str()),
        Key::Index(i) => value.get_mut(*i),
    })
}

/// `.user.name`, `.items[0]`.
fn json_path(keys: &[Key]) -> String {
    keys.iter()
        .map(|key| match key {
            Key::Field(name) => format!(".{name}"),
            Key::Index(i) => format!("[{i}]"),
        })
        .collect()
}

static SQL_ERROR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)you have an error in your sql syntax|warning: mysql|unclosed quotation mark after the character string|quoted string not properly terminated|syntax error at or near|pg::syntaxerror|sqlite3?::|sqlite_error|sqlstate\[|ora-\d{5}|microsoft ole db provider for sql server|odbc [a-z ]*driver|unterminated quoted string",
    )
    .expect("a valid pattern")
});

static STACK_TRACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"Traceback \(most recent call last\)|\bat [\w$.]+\([\w]+\.java:\d+\)|goroutine \d+ \[running\]|\bat .+ \(.+\.[jt]s:\d+:\d+\)|System\.[\w.]+Exception|Exception in thread|Fatal error: .+ on line \d+|Whitelabel Error Page|\.rb:\d+:in `",
    )
    .expect("a valid pattern")
});

static FILE_CONTENTS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"root:[^:\n]*:0:0:|\[fonts\]|\[extensions\]").expect("a valid pattern"));

static COMMAND_OUTPUT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"uid=\d+\([\w-]+\) gid=\d+").expect("a valid pattern"));

/// What `response` to `payload` at `location` shows that `baseline` didn't.
fn detect(location: &str, payload: &Payload, response: &Response, baseline: &Response) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut add = |severity, title: &str, rule: &str, detail: String, at: Option<usize>| {
        let shown: String = payload.text.chars().take(80).collect();
        let evidence = match at {
            Some(at) => format!("{shown}\n{}: {}", response.status, snippet(&response.body, at)),
            None => format!("{shown}\n{}", response.status),
        };
        findings.push(Finding {
            severity,
            title: title.into(),
            rule: rule.into(),
            location: location.into(),
            detail,
            evidence,
            repro: String::new(),
        });
    };
    // A pattern counts only where the baseline didn't have it already.
    let new_match = |pattern: &Regex| {
        pattern.find(&response.body).filter(|_| !pattern.is_match(&baseline.body)).map(|found| found.start())
    };

    if response.status >= 500 && baseline.status < 500 {
        add(Severity::Medium, "Server error", "fuzz.server-error", format!("Got {}", response.status), None);
    }
    if let Some(at) = new_match(&SQL_ERROR) {
        add(
            Severity::High,
            "SQL error in response",
            "fuzz.sql-error",
            "A database error reached the client".into(),
            Some(at),
        );
    }
    if let Some(at) = new_match(&STACK_TRACE) {
        add(
            Severity::Medium,
            "Stack trace in response",
            "fuzz.stack-trace",
            "Internals reached the client".into(),
            Some(at),
        );
    }
    if payload.kind == PayloadKind::Xss
        && is_html(response)
        && let Some(at) = response.body.find(&payload.text)
    {
        add(
            Severity::High,
            "Reflected XSS",
            "fuzz.reflected-xss",
            "Markup sent back unescaped as HTML".into(),
            Some(at),
        );
    }
    if payload.kind == PayloadKind::Traversal
        && let Some(at) = new_match(&FILE_CONTENTS)
    {
        add(Severity::Critical, "Path traversal", "fuzz.path-traversal", "A system file came back".into(), Some(at));
    }
    if payload.kind == PayloadKind::Command
        && let Some(at) = new_match(&COMMAND_OUTPUT)
    {
        add(Severity::Critical, "Command injection", "fuzz.command-injection", "A shell command ran".into(), Some(at));
    }
    if payload.sleeps && response.time_ms >= baseline.time_ms + SLOWER_MS {
        let detail = format!("{}ms, against {}ms without it", response.time_ms, baseline.time_ms);
        add(Severity::Medium, "Slowed by a sleep payload", "fuzz.time-based", detail, None);
    }
    findings
}

/// A request the server never answered: it dropped the connection or timed out.
fn no_response(location: &str, payload: &Payload, error: &str) -> Finding {
    Finding {
        severity: Severity::Medium,
        title: "No response".into(),
        rule: "fuzz.no-response".into(),
        location: location.into(),
        detail: error.into(),
        evidence: payload.text.chars().take(80).collect(),
        repro: String::new(),
    }
}

/// A curl command that sends `response`'s request again, secrets masked.
fn curl(response: &Response) -> String {
    let quote = |text: &str| format!("'{}'", text.replace('\'', r"'\''"));
    let mut parts = vec!["curl".to_string(), "-X".into(), response.method.clone(), quote(&mask_url(&response.url))];
    for (name, value) in &response.sent.headers {
        let value = if is_secret(name) { MASK } else { value.as_str() };
        parts.extend(["-H".into(), quote(&format!("{name}: {value}"))]);
    }
    if let Some(body) = response.sent.body.as_deref().filter(|body| !body.is_empty()) {
        parts.extend(["--data-raw".into(), quote(&mask_body(body))]);
    }
    parts.join(" ")
}

fn is_secret(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SECRET_NAMES.iter().any(|part| lower.contains(part))
}

/// `url` with the values of secret-named query pairs masked.
fn mask_url(url: &str) -> String {
    let Some((base, rest)) = url.split_once('?') else { return url.to_string() };
    let (query, fragment) = rest.split_once('#').map_or((rest, None), |(query, fragment)| (query, Some(fragment)));
    let fragment = fragment.map_or_default(|fragment| format!("#{fragment}"));
    format!("{base}?{}{fragment}", mask_pairs(query))
}

/// `name=value&…` with the values of secret-named pairs masked.
fn mask_pairs(pairs: &str) -> String {
    pairs
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((name, _)) if is_secret(name) => format!("{name}={MASK}"),
            _ => pair.to_string(),
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// `body` with secret-named fields masked, if it's JSON or a form.
fn mask_body(body: &str) -> String {
    if let Ok(mut json) = serde_json::from_str::<Value>(body) {
        return if mask_json(&mut json) { json.to_string() } else { body.to_string() };
    }
    let is_form = !body.contains(char::is_whitespace) && body.contains('=');
    if is_form { mask_pairs(body) } else { body.to_string() }
}

/// Mask the values of secret-named fields in `json`; whether any were.
fn mask_json(json: &mut Value) -> bool {
    match json {
        Value::Object(map) => map.iter_mut().fold(false, |masked, (name, value)| {
            if is_secret(name) && !value.is_null() {
                *value = Value::String(MASK.into());
                true
            } else {
                mask_json(value) || masked
            }
        }),
        Value::Array(items) => {
            let mut masked = false;
            for item in items {
                masked |= mask_json(item);
            }
            masked
        }
        _ => false,
    }
}

fn is_html(response: &Response) -> bool {
    response
        .headers
        .iter()
        .any(|(name, value)| name.eq_ignore_ascii_case("content-type") && value.to_ascii_lowercase().contains("html"))
}

/// Up to [`SNIPPET_CHARS`] of `body` around byte `at`.
fn snippet(body: &str, at: usize) -> String {
    let start = body.char_indices().map(|(i, _)| i).take_while(|&i| i + 60 <= at).last().unwrap_or(0);
    body[start..].chars().take(SNIPPET_CHARS).collect::<String>().replace(['\r', '\n'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::Header;
    use crate::scenario::{FormField, Param};

    fn scenario() -> Scenario {
        let mut scenario = Scenario::default();
        scenario.http.url = "{{base}}/orders/:id?page=2&sort=name#top".into();
        scenario.http.params =
            vec![Param { name: "id".into(), value: "{{id}}".into(), kind: ParamKind::Path, disabled: false }];
        scenario.http.headers = vec![Header::new("Authorization", "Bearer x"), Header::new("X-Tenant", "acme")];
        scenario.http.body = Some(Body::Typed(TypedBody::Json {
            data: r#"{"user": {"name": "{{name}}", "tags": ["a"]}, "n": 1, "empty": {}}"#.into(),
        }));
        scenario
    }

    fn locations(slots: &[Slot]) -> Vec<&str> {
        slots.iter().map(|slot| slot.location.as_str()).collect()
    }

    #[test]
    fn finds_each_value_of_the_targets() {
        let all = BTreeSet::from([FuzzTarget::Query, FuzzTarget::Path, FuzzTarget::Headers, FuzzTarget::Body]);
        assert_eq!(
            locations(&slots(&scenario(), &all)),
            ["query.page", "query.sort", "path.id", "header.X-Tenant", "body.user.name", "body.user.tags[0]", "body.n"]
        );
        assert_eq!(locations(&slots(&scenario(), &Fuzz::default().targets())).len(), 6, "no headers by default");

        let mut form = Scenario::default();
        form.http.body = Some(Body::Typed(TypedBody::FormUrlencoded {
            data: vec![FormField { name: "q".into(), value: "x".into(), disabled: false }],
        }));
        assert_eq!(locations(&slots(&form, &all)), ["form.q"]);
    }

    #[test]
    fn mutations_start_from_each_value_and_repeat_with_the_seed() {
        let slots = slots(&scenario(), &BTreeSet::from([FuzzTarget::Query, FuzzTarget::Body]));
        let originals: Vec<&Value> = slots.iter().map(|slot| &slot.original).collect();
        assert_eq!(originals[0], &Value::from("2"), "query.page");
        assert_eq!(originals[4], &Value::from(1), "body.n keeps its type");

        let fuzz = Fuzz { payloads: vec![PayloadKind::Mutate], iterations: 5, ..Fuzz::default() };
        let cases = fuzz.cases(&slots, 42);
        assert_eq!(cases.len(), 5 * slots.len());
        assert!(cases.iter().all(|(_, payload)| payload.kind == PayloadKind::Mutate));
        assert_eq!(cases, fuzz.cases(&slots, 42), "the same seed, the same inputs");

        let both = Fuzz { payloads: vec![PayloadKind::Sqli, PayloadKind::Mutate], iterations: 5, ..Fuzz::default() };
        assert_eq!(both.cases(&slots, 42).len(), (8 + 5) * slots.len());
    }

    #[test]
    fn repro_is_a_curl_command_with_secrets_masked() {
        let mut response =
            Response { method: "POST".into(), url: "https://x.test/q?a='1'".into(), ..Response::default() };
        response.sent.headers = vec![
            ("authorization".into(), "Bearer abc".into()),
            ("x-api-key".into(), "k".into()),
            ("accept".into(), "*/*".into()),
        ];
        response.sent.body = Some(r#"{"q":"it's"}"#.into());
        assert_eq!(
            curl(&response),
            r#"curl -X POST 'https://x.test/q?a='\''1'\''' -H 'authorization: ••••' -H 'x-api-key: ••••' -H 'accept: */*' --data-raw '{"q":"it'\''s"}'"#
        );
    }

    #[test]
    fn repro_masks_secret_query_pairs_and_body_fields() {
        let mut response =
            Response { method: "POST".into(), url: "https://x.test/q?api_key=k&q=1#top".into(), ..Response::default() };
        response.sent.body = Some(r#"{"user":"ann","password":"hunter2","items":[{"token":"t"}]}"#.into());
        assert_eq!(
            curl(&response),
            r#"curl -X POST 'https://x.test/q?api_key=••••&q=1#top' --data-raw '{"user":"ann","password":"••••","items":[{"token":"••••"}]}'"#
        );
        response.sent.body = Some("user=ann&password=hunter2".into());
        assert!(curl(&response).ends_with("--data-raw 'user=ann&password=••••'"));
    }

    #[test]
    fn puts_a_payload_in_one_place() {
        let slots = slots(&scenario(), &BTreeSet::from([FuzzTarget::Query, FuzzTarget::Path, FuzzTarget::Body]));
        let quote = Payload::text(PayloadKind::Sqli, "' OR 1");
        let mut changed = scenario();
        slots[1].apply(&mut changed, &quote);
        assert_eq!(changed.http.url.0, "{{base}}/orders/:id?page=2&sort=%27%20OR%201#top");
        slots[2].apply(&mut changed, &quote);
        assert_eq!(changed.http.params[0].value.0, "%27%20OR%201");
        slots[4].apply(&mut changed, &Payload::typed("null", Value::Null));
        let Some(Body::Typed(TypedBody::Json { data })) = &changed.http.body else { panic!() };
        assert_eq!(data.0, r#"{"user":{"name":"{{name}}","tags":[null]},"n":1,"empty":{}}"#);
    }

    #[test]
    fn the_overflow_number_goes_into_json_as_is() {
        let slots = slots(&scenario(), &BTreeSet::from([FuzzTarget::Body]));
        let overflow = payloads(&BTreeSet::from([PayloadKind::Types]))
            .into_iter()
            .find(|payload| payload.text == "18446744073709551616")
            .unwrap();
        let mut changed = scenario();
        slots[2].apply(&mut changed, &overflow);
        let Some(Body::Typed(TypedBody::Json { data })) = &changed.http.body else { panic!() };
        assert_eq!(data.0, r#"{"user":{"name":"{{name}}","tags":["a"]},"n":18446744073709551616,"empty":{}}"#);
    }

    #[test]
    fn repeated_names_get_an_index_and_content_headers_are_kept() {
        let mut scenario = scenario();
        scenario.http.url = "https://x.test/?id=1&id=2&q=3".into();
        scenario.http.headers = vec![Header::new("Content-Encoding", "gzip"), Header::new("X-Tenant", "acme")];
        let targets = BTreeSet::from([FuzzTarget::Query, FuzzTarget::Headers]);
        assert_eq!(
            locations(&slots(&scenario, &targets)),
            ["query.id[0]", "query.id[1]", "query.q", "header.X-Tenant"]
        );
    }

    #[test]
    fn headers_only_take_payloads_they_can_carry() {
        let header = Slot { place: Place::Header(0), location: "header.X".into(), original: Value::from("x") };
        assert!(header.takes(&Payload::text(PayloadKind::Sqli, "'")));
        assert!(!header.takes(&Payload::text(PayloadKind::Overflow, "\0")));
    }

    fn response(status: u16, body: &str, html: bool) -> Response {
        let headers = if html { vec![("content-type".into(), "text/html".into())] } else { Vec::new() };
        Response { status, body: body.into(), headers, ..Response::default() }
    }

    fn rules(findings: &[Finding]) -> Vec<&str> {
        findings.iter().map(|finding| finding.rule.as_str()).collect()
    }

    #[test]
    fn detects_what_the_payloads_give_away() {
        let baseline = response(200, "[]", false);
        let quote = Payload::text(PayloadKind::Sqli, "'");
        let sql = response(500, "ERROR: unterminated quoted string at or near \"'\"", false);
        let found = detect("query.q", &quote, &sql, &baseline);
        assert_eq!(rules(&found), ["fuzz.server-error", "fuzz.sql-error"]);
        assert!(found[1].evidence.starts_with("'\n500: ERROR: unterminated"), "{}", found[1].evidence);

        let xss = Payload::text(PayloadKind::Xss, "<svg/onload=alert(1337)>");
        let reflected = response(200, "<p>No results for <svg/onload=alert(1337)></p>", true);
        assert_eq!(rules(&detect("query.q", &xss, &reflected, &baseline)), ["fuzz.reflected-xss"]);
        let as_json = response(200, "<svg/onload=alert(1337)>", false);
        assert_eq!(detect("query.q", &xss, &as_json, &baseline), [], "JSON isn't rendered");

        let passwd = response(200, "root:x:0:0:root:/root:/bin/bash", false);
        let traversal = Payload::text(PayloadKind::Traversal, "/etc/passwd");
        assert_eq!(rules(&detect("path.file", &traversal, &passwd, &baseline)), ["fuzz.path-traversal"]);

        let mut slow = response(200, "[]", false);
        slow.time_ms = 6100;
        let sleep = Payload::sleeping(PayloadKind::Command, ";sleep SECONDS");
        assert_eq!(sleep.text, ";sleep 6");
        assert_eq!(rules(&detect("form.q", &sleep, &slow, &baseline)), ["fuzz.time-based"]);
    }

    #[test]
    fn a_pattern_the_baseline_already_had_is_not_a_finding() {
        let trace = "Traceback (most recent call last): ...";
        let found = detect(
            "query.q",
            &Payload::text(PayloadKind::Sqli, "'"),
            &response(200, trace, false),
            &response(200, trace, false),
        );
        assert_eq!(found, []);
    }

    #[test]
    fn every_fixed_kind_has_payloads() {
        for kind in PayloadKind::ALL.into_iter().filter(|kind| *kind != PayloadKind::Mutate) {
            assert!(!payloads(&BTreeSet::from([kind])).is_empty(), "{kind:?}");
        }
        assert!(payloads(&BTreeSet::from([PayloadKind::Mutate])).is_empty(), "made per value instead");
    }
}
