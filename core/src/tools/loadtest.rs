//! `- tool: loadtest`: send a request many times at once and measure how
//! fast the answers come. Fails on its metric `checks`, e.g. `p95 lt 500`.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use super::findings::{Exports, export};
use super::stats::{mean, per_second, percent, percentile, sorted};
use super::wait::MAX_SECONDS;
use super::{Metric, Tool, ToolCx, ToolOutput, is_zero, metrics_verdict, request_step, unchanged};
use crate::collection::LenientString;
use crate::scenario::Assertion;
use crate::{Error, Result};

/// Requests and senders when not set, and the most of each.
const DEFAULT_REQUESTS: u32 = 100;
const DEFAULT_CONCURRENCY: u32 = 10;
pub const MAX_REQUESTS: u32 = 10_000;
pub const MAX_CONCURRENCY: u32 = 100;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Loadtest {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub request: LenientString,
    /// Variables for this request only, as a request step's `with`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub with: BTreeMap<String, LenientString>,
    /// How many to send; 100 when neither this nor `duration` is set.
    #[serde(skip_serializing_if = "is_zero")]
    pub requests: u32,
    /// Seconds to keep sending for.
    #[serde(skip_serializing_if = "is_zero")]
    pub duration: u32,
    /// How many to have in flight at once; 10 when not set.
    #[serde(skip_serializing_if = "is_zero")]
    pub concurrency: u32,
    /// Limits on the metrics: `p95 lt 500`, `error_rate lt 1`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<Assertion>,
    /// Metrics for later steps: `latency: p95`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub set: BTreeMap<String, LenientString>,
}

/// What `set` can name.
const EXPORTS: Exports = Exports {
    metrics: &["requests", "rps", "min", "p50", "p90", "p95", "p99", "max", "mean", "error_rate", "non2xx"],
    findings: false,
};

/// What one request got: its status and time, or nothing if it wasn't answered.
type Sample = Option<(u16, u64)>;

impl Tool for Loadtest {
    fn title(&self) -> String {
        format!("Load test {}", self.request.trim())
    }

    fn templates(&mut self) -> Vec<&mut String> {
        self.checks.iter_mut().filter_map(|check| check.value.as_mut().map(|value| &mut value.0)).collect()
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        EXPORTS.check(&self.set)?;
        if self.requests > MAX_REQUESTS {
            return Err(Error::Tool(format!("Send at most {MAX_REQUESTS} requests")));
        }
        if self.duration > MAX_SECONDS {
            return Err(Error::Tool(format!("Run for at most {MAX_SECONDS} seconds")));
        }
        if self.concurrency > MAX_CONCURRENCY {
            return Err(Error::Tool(format!("Send at most {MAX_CONCURRENCY} at once")));
        }
        let limit = match (self.requests, self.duration) {
            (0, 0) => DEFAULT_REQUESTS,
            // Only the clock stops it.
            (0, _) => u32::MAX,
            (requests, _) => requests,
        };
        let concurrency = if self.concurrency == 0 { DEFAULT_CONCURRENCY } else { self.concurrency }.min(limit);
        let started = Instant::now();
        let deadline = (self.duration > 0).then(|| started + Duration::from_secs(u64::from(self.duration)));
        let step = request_step(&self.request, &self.with);
        let (next, done) = (AtomicU32::new(0), AtomicU32::new(0));
        let samples: Mutex<Vec<Sample>> = Mutex::new(Vec::new());
        let first_error: Mutex<Option<String>> = Mutex::new(None);

        let shared: &ToolCx<'_> = cx;
        let sender = || async {
            loop {
                if deadline.is_some_and(|deadline| Instant::now() >= deadline)
                    || next.fetch_add(1, Ordering::SeqCst) >= limit
                {
                    break;
                }
                let sample = match shared.runner.probe(&step, &unchanged).await {
                    Ok(response) => Some((response.status, response.time_ms)),
                    Err(error) => {
                        first_error.lock().unwrap().get_or_insert_with(|| error.to_string());
                        None
                    }
                };
                samples.lock().unwrap().push(sample);
                let done = done.fetch_add(1, Ordering::SeqCst) + 1;
                match deadline {
                    // Seconds gone, of the run's.
                    Some(_) => {
                        shared.progress(started.elapsed().as_secs().try_into().unwrap_or(u32::MAX), self.duration);
                    }
                    None => shared.progress(done, limit),
                }
            }
        };
        join_all((0..concurrency).map(|_| sender())).await;

        let samples = samples.into_inner().unwrap();
        if samples.iter().all(Option::is_none) {
            let why = first_error.into_inner().unwrap().unwrap_or_else(|| "nothing was sent".into());
            return Err(Error::Tool(format!("No responses: {why}")));
        }
        let metrics = summarise(&samples, started.elapsed().as_secs_f64());
        let text = format!("{} requests, {} at once", samples.len(), concurrency);
        let output = ToolOutput { metrics, ..ToolOutput::text(text) };
        export(cx, &self.set, &output)?;
        Ok(output)
    }

    fn verdict(&self, output: &ToolOutput) -> Option<String> {
        metrics_verdict(&self.checks, &output.metrics)
    }
}

/// The metrics of a run that took `seconds`.
#[allow(clippy::cast_precision_loss)]
fn summarise(samples: &[Sample], seconds: f64) -> Vec<Metric> {
    let times = sorted(samples.iter().flatten().map(|(_, ms)| *ms as f64).collect());
    let failed = samples.iter().filter(|sample| sample.is_none_or(|(status, _)| status >= 500)).count();
    let non_2xx = samples.iter().flatten().filter(|(status, _)| !(200..300).contains(status)).count();
    let ms = |name: &str, label: &str, value: f64| Metric::new(name, label, value, "ms");
    vec![
        Metric::new("requests", "Requests", samples.len() as f64, ""),
        Metric::new("rps", "Rate", per_second(samples.len(), seconds), "/s"),
        ms("min", "Min", times.first().copied().unwrap_or(f64::NAN)),
        ms("p50", "P50", percentile(&times, 50.0)),
        ms("p90", "P90", percentile(&times, 90.0)),
        ms("p95", "P95", percentile(&times, 95.0)),
        ms("p99", "P99", percentile(&times, 99.0)),
        ms("max", "Max", times.last().copied().unwrap_or(f64::NAN)),
        ms("mean", "Mean", mean(&times)),
        Metric::new("error_rate", "Error rate", percent(failed, samples.len()), "%"),
        Metric::new("non2xx", "Not 2xx", non_2xx as f64, ""),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarises_times_and_failures() {
        let samples = [Some((200, 10)), Some((200, 30)), Some((404, 20)), Some((503, 40)), None];
        let metrics = summarise(&samples, 2.0);
        let value = |name: &str| metrics.iter().find(|metric| metric.name == name).unwrap().value;
        assert_eq!(value("requests"), 5.0);
        assert_eq!(value("rps"), 2.5);
        assert_eq!((value("min"), value("p50"), value("max")), (10.0, 20.0, 40.0));
        assert_eq!(value("mean"), 25.0);
        assert_eq!(value("error_rate"), 40.0, "a 5xx and an unanswered request");
        assert_eq!(value("non2xx"), 2.0);
        let names: Vec<&str> = metrics.iter().map(|metric| metric.name.as_str()).collect();
        assert_eq!(names, EXPORTS.metrics, "set can name every metric");
    }
}
