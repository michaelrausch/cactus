//! `- tool: ratelimit`: send a request more times than it should allow and
//! check the server starts refusing (429), says when to retry, and stops
//! refusing once that time has passed.

use std::collections::BTreeMap;
use std::time::Duration;

use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use super::findings::{Exports, export};
use super::stats::per_second;
use super::wait::MAX_SECONDS;
use super::{
    FailOn, Finding, Metric, Severity, Tool, ToolCx, ToolOutput, findings_verdict, is_zero, request_step, unchanged,
};
use crate::collection::LenientString;
use crate::collection::model::is_false;
use crate::http::Response;
use crate::{Error, Result};

/// Requests sent when `limit` isn't set, and the most it sends.
const DEFAULT_REQUESTS: u32 = 100;
pub const MAX_REQUESTS: u32 = 1000;
pub const MAX_CONCURRENCY: u32 = 100;
/// Seconds to wait before trying again when a 429 doesn't say.
const DEFAULT_RETRY_AFTER: u64 = 60;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Ratelimit {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub request: LenientString,
    /// Variables for this request only, as a request step's `with`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub with: BTreeMap<String, LenientString>,
    /// The most requests that should get through before it's limited.
    #[serde(skip_serializing_if = "is_zero")]
    pub limit: u32,
    /// How many to send; three times `limit`, or 100, when not set.
    #[serde(skip_serializing_if = "is_zero")]
    pub requests: u32,
    /// How many to have in flight at once; 1 when not set.
    #[serde(skip_serializing_if = "is_zero")]
    pub concurrency: u32,
    /// Once limited, wait as long as it says and check it lets one through.
    #[serde(skip_serializing_if = "is_false")]
    pub reset: bool,
    #[serde(skip_serializing_if = "FailOn::is_default")]
    pub fail_on: FailOn,
    /// Metrics or findings for later steps: `allowed: allowed`, `gaps: findings`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub set: BTreeMap<String, LenientString>,
}

/// What `set` can name. `limited_at` is empty when nothing was limited.
const EXPORTS: Exports = Exports { metrics: &["sent", "allowed", "limited_at", "rps"], findings: true };

/// What one response said about limiting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Seen {
    status: u16,
    /// `Retry-After`, as written.
    retry_after: Option<String>,
    /// Any `RateLimit-*` or `X-RateLimit-*` header.
    limit_headers: bool,
}

impl Seen {
    fn of(response: &Response) -> Self {
        let header = |name: &str| response.headers.iter().find(|(key, _)| key.eq_ignore_ascii_case(name));
        let limit_headers = response.headers.iter().any(|(key, _)| {
            let key = key.to_ascii_lowercase();
            key.starts_with("ratelimit") || key.starts_with("x-ratelimit")
        });
        Self {
            status: response.status,
            retry_after: header("retry-after").map(|(_, value)| value.clone()),
            limit_headers,
        }
    }

    /// A 429, or a 503 that says when to retry.
    fn limited(&self) -> bool {
        self.status == 429 || (self.status == 503 && self.retry_after.is_some())
    }

    fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Seconds `Retry-After` asks for, when it's a number.
    fn retry_seconds(&self) -> Option<u64> {
        self.retry_after.as_deref()?.trim().parse().ok()
    }
}

impl Ratelimit {
    fn requests(&self) -> u32 {
        match (self.requests, self.limit) {
            (0, 0) => DEFAULT_REQUESTS,
            (0, limit) => limit.saturating_mul(3).min(MAX_REQUESTS),
            (requests, _) => requests,
        }
    }
}

impl Tool for Ratelimit {
    fn title(&self) -> String {
        format!("Rate limit {}", self.request.trim())
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        EXPORTS.check(&self.set)?;
        let total = self.requests();
        if total > MAX_REQUESTS {
            return Err(Error::Tool(format!("Send at most {MAX_REQUESTS} requests")));
        }
        if total <= self.limit {
            return Err(Error::Tool("Send more requests than the limit".into()));
        }
        if self.concurrency > MAX_CONCURRENCY {
            return Err(Error::Tool(format!("Send at most {MAX_CONCURRENCY} at once")));
        }
        let step = request_step(&self.request, &self.with);
        let started = Instant::now();
        let mut seen = Vec::new();
        let mut first_error = None;
        let mut sends = stream::iter(0..total)
            .map(|_| cx.runner.probe(&step, &unchanged))
            .buffered(self.concurrency.max(1) as usize);
        while let Some(outcome) = sends.next().await {
            match outcome {
                Ok(response) => seen.push(Some(Seen::of(&response))),
                Err(error) => {
                    first_error.get_or_insert_with(|| error.to_string());
                    seen.push(None);
                }
            }
            cx.progress(u32::try_from(seen.len()).unwrap_or(u32::MAX), total);
        }
        drop(sends);
        if seen.iter().all(Option::is_none) {
            return Err(Error::Tool(format!("No responses: {}", first_error.unwrap_or_default())));
        }
        let seconds = started.elapsed().as_secs_f64();
        let location = self.request.trim();
        let (mut findings, metrics) = analyse(&seen, self.limit, seconds, location);

        let last_limited = seen.iter().flatten().rfind(|seen| seen.limited());
        if let (true, Some(limited)) = (self.reset, last_limited) {
            let wait = limited.retry_seconds().unwrap_or(DEFAULT_RETRY_AFTER).min(u64::from(MAX_SECONDS));
            // A second more, so a window that ends on the second has ended.
            tokio::time::sleep(Duration::from_secs(wait + 1)).await;
            // A failed retry is a finding too, so what was already found is kept.
            let still = match cx.runner.probe(&step, &unchanged).await {
                Ok(response) => {
                    Seen::of(&response).limited().then(|| format!("Got {} again {wait}s later", response.status))
                }
                Err(error) => Some(format!("No answer {wait}s later: {error}")),
            };
            if let Some(detail) = still {
                findings.push(Finding {
                    severity: Severity::Low,
                    title: "Still limited after waiting".into(),
                    rule: "ratelimit.no-reset".into(),
                    location: location.into(),
                    detail,
                    ..Finding::default()
                });
            }
        }
        let text = match seen.iter().flatten().position(Seen::limited) {
            Some(at) => format!("Limited after {at} of {total}"),
            None => format!("None of {total} limited"),
        };
        let output = ToolOutput { findings: Some(findings), metrics, ..ToolOutput::text(text) };
        export(cx, &self.set, &output)?;
        Ok(output)
    }

    fn verdict(&self, output: &ToolOutput) -> Option<String> {
        findings_verdict(self.fail_on, output.findings.as_deref())
    }
}

/// What `seen`, in the order sent, says about the limit: findings and
/// metrics. `None` is a request that got no response.
#[allow(clippy::cast_precision_loss)]
fn analyse(seen: &[Option<Seen>], limit: u32, seconds: f64, location: &str) -> (Vec<Finding>, Vec<Metric>) {
    let finding = |severity, title: &str, rule: &str, detail: String| Finding {
        severity,
        title: title.into(),
        rule: rule.into(),
        location: location.into(),
        detail,
        ..Finding::default()
    };
    let answered: Vec<&Seen> = seen.iter().flatten().collect();
    let mut metrics = vec![Metric::new("sent", "Sent", seen.len() as f64, "")];
    let Some(first) = answered.iter().position(|seen| seen.limited()) else {
        let detail = format!("None of {} responses was a 429", answered.len());
        metrics.push(Metric::new("allowed", "Allowed", answered.len() as f64, ""));
        metrics.push(Metric::new("rps", "Rate", per_second(seen.len(), seconds), "/s"));
        return (vec![finding(Severity::High, "No rate limit", "ratelimit.missing", detail)], metrics);
    };
    let mut findings = Vec::new();
    if limit > 0 && first > limit as usize {
        let detail = format!("Limited after {first} requests; expected at most {limit}");
        findings.push(finding(Severity::Medium, "Limit higher than expected", "ratelimit.too-high", detail));
    }
    let limited: Vec<&&Seen> = answered.iter().filter(|seen| seen.limited()).collect();
    let silent = limited.iter().filter(|seen| seen.retry_after.is_none() && !seen.limit_headers).count();
    if silent > 0 {
        let detail = format!("{silent} of {} limited responses had no Retry-After or RateLimit headers", limited.len());
        findings.push(finding(Severity::Low, "No Retry-After", "ratelimit.no-retry-after", detail));
    }
    let after = &answered[first + 1..];
    let let_through = after.iter().filter(|seen| seen.ok()).count();
    if let_through > 0 {
        let detail = format!("{let_through} of {} requests after the first 429 got a 2xx", after.len());
        findings.push(finding(Severity::Low, "Allowed again while limited", "ratelimit.bypass", detail));
    }
    metrics.push(Metric::new("allowed", "Allowed", first as f64, ""));
    metrics.push(Metric::new("limited_at", "Limited at", (first + 1) as f64, ""));
    metrics.push(Metric::new("rps", "Rate", per_second(seen.len(), seconds), "/s"));
    (findings, metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What was seen of a request, as `analyse` takes it.
    #[allow(clippy::unnecessary_wraps)]
    fn ok() -> Option<Seen> {
        Some(Seen { status: 200, ..Seen::default() })
    }

    #[allow(clippy::unnecessary_wraps)]
    fn limited(retry_after: Option<&str>) -> Option<Seen> {
        Some(Seen { status: 429, retry_after: retry_after.map(Into::into), limit_headers: false })
    }

    fn rules(findings: &[Finding]) -> Vec<&str> {
        findings.iter().map(|finding| finding.rule.as_str()).collect()
    }

    #[test]
    fn a_limit_that_holds_and_says_when_to_retry_is_clean() {
        let seen = [ok(), ok(), ok(), limited(Some("30")), limited(Some("30"))];
        let (findings, metrics) = analyse(&seen, 3, 1.0, "login.yml");
        assert_eq!(findings, []);
        let value = |name: &str| metrics.iter().find(|metric| metric.name == name).unwrap().value;
        assert_eq!((value("sent"), value("allowed"), value("limited_at")), (5.0, 3.0, 4.0));
    }

    #[test]
    fn finds_a_missing_or_loose_limit() {
        let (findings, _) = analyse(&[ok(), ok(), None], 1, 1.0, "login.yml");
        assert_eq!(rules(&findings), ["ratelimit.missing"]);
        assert_eq!(findings[0].detail, "None of 2 responses was a 429");

        let seen = [ok(), ok(), ok(), limited(None), ok(), limited(None)];
        let (findings, _) = analyse(&seen, 2, 1.0, "login.yml");
        assert_eq!(rules(&findings), ["ratelimit.too-high", "ratelimit.no-retry-after", "ratelimit.bypass"]);
        assert_eq!(findings[0].location, "login.yml");
    }

    #[test]
    fn a_503_counts_only_when_it_says_when_to_retry() {
        let busy = Seen { status: 503, ..Seen::default() };
        assert!(!busy.limited());
        assert!(Seen { retry_after: Some("5".into()), ..busy }.limited());
    }

    #[test]
    fn sends_three_times_the_limit_by_default() {
        assert_eq!(Ratelimit { limit: 10, ..Ratelimit::default() }.requests(), 30);
        assert_eq!(Ratelimit::default().requests(), DEFAULT_REQUESTS);
        assert_eq!(Ratelimit { limit: 900, ..Ratelimit::default() }.requests(), MAX_REQUESTS);
    }
}
