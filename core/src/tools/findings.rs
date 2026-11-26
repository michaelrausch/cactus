//! What security tools report: findings with a severity, and metrics. A
//! step's `fail_on` decides which findings fail it; `checks` on metric
//! names decide the same for numbers.
//!
//! ```yaml
//! - tool: headers
//!   fail_on: medium        # never, info, low, medium, high (default), critical
//! - tool: loadtest
//!   checks:
//!     - {expression: p95, operator: lt, value: "500"}
//! ```

use std::cmp::Reverse;
use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::http::check_value;
use crate::scenario::Assertion;

/// How bad a finding is. Ordered, so `Critical > High`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Severity {
    #[default]
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// One thing a tool found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Finding {
    pub severity: Severity,
    /// What it is: "Missing Strict-Transport-Security".
    pub title: String,
    /// A stable id, e.g. `headers.hsts`, so runs can be compared.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rule: String,
    /// Where: a URL, header, parameter or JSON path.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub location: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// What was sent or seen, e.g. the payload and a response snippet.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub evidence: String,
}

impl Finding {
    pub fn key(&self) -> FindingKey {
        FindingKey { severity: self.severity, rule: self.rule.clone(), location: self.location.clone() }
    }

    /// One line: `high  headers.hsts  https://…  Missing HSTS`.
    pub fn line(&self) -> String {
        let parts = [self.severity.as_str(), &self.rule, &self.location, &self.title];
        parts.iter().filter(|part| !part.is_empty()).copied().collect::<Vec<_>>().join("  ")
    }
}

/// A finding without its text, as the run log keeps it. `rule` and
/// `location` identify it between runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FindingKey {
    pub severity: Severity,
    pub rule: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub location: String,
}

impl FindingKey {
    /// Whether `self`, from an earlier run, already reported `other`: the
    /// same rule and location, at least as severe.
    pub fn covers(&self, other: &Self) -> bool {
        self.rule == other.rule && self.location == other.location && self.severity >= other.severity
    }
}

/// One number a tool measured.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Metric {
    /// What checks name it by: `p95`, `rps`, `errors`.
    pub name: String,
    /// How it's shown: "P95".
    pub label: String,
    #[serde(deserialize_with = "nan_if_null")]
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub value: f64,
    /// `ms`, `%`, `/s`, or empty for a count.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub unit: String,
}

impl Metric {
    pub fn new(name: &str, label: &str, value: f64, unit: &str) -> Self {
        Self { name: name.into(), label: label.into(), value, unit: unit.into() }
    }

    /// The value with its unit: `412ms`, `2.5%`, `120/s`, `1500`.
    pub fn shown(&self) -> String {
        let value = if self.value.fract() == 0.0 { format!("{}", self.value) } else { format!("{:.1}", self.value) };
        format!("{value}{}", self.unit)
    }
}

/// JSON has no NaN or infinity: `serde_json` writes them as `null`, which
/// reads back as NaN.
fn nan_if_null<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    Ok(Option::<f64>::deserialize(deserializer)?.unwrap_or(f64::NAN))
}

/// The lowest severity that fails a step, or never.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum FailOn {
    Never,
    Info,
    Low,
    Medium,
    #[default]
    High,
    Critical,
}

impl FailOn {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    fn threshold(self) -> Option<Severity> {
        match self {
            Self::Never => None,
            Self::Info => Some(Severity::Info),
            Self::Low => Some(Severity::Low),
            Self::Medium => Some(Severity::Medium),
            Self::High => Some(Severity::High),
            Self::Critical => Some(Severity::Critical),
        }
    }
}

/// Why `findings` fail a step with `fail_on`: "2 high, 1 critical findings".
pub fn findings_verdict(fail_on: FailOn, findings: Option<&[Finding]>) -> Option<String> {
    let threshold = fail_on.threshold()?;
    // Most severe first.
    let mut counts: BTreeMap<Reverse<Severity>, usize> = BTreeMap::new();
    for finding in findings.unwrap_or_default().iter().filter(|finding| finding.severity >= threshold) {
        *counts.entry(Reverse(finding.severity)).or_default() += 1;
    }
    let total: usize = counts.values().sum();
    if total == 0 {
        return None;
    }
    let parts: Vec<String> =
        counts.iter().map(|(Reverse(severity), count)| format!("{count} {}", severity.as_str())).collect();
    Some(format!("{} finding{}", parts.join(", "), if total == 1 { "" } else { "s" }))
}

/// The first of `checks` that `metrics` fail, by metric name.
pub fn metrics_verdict(checks: &[Assertion], metrics: &[Metric]) -> Option<String> {
    checks.iter().filter(|check| !check.disabled).find_map(|assertion| {
        let name = assertion.expression.trim();
        let value = metrics.iter().find(|metric| metric.name == name).map(|metric| Value::from(metric.value));
        let check = check_value(assertion, value.as_ref());
        (!check.ok).then(|| format!("{}: {}", check.name, check.detail))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(severity: Severity) -> Finding {
        Finding { severity, title: "x".into(), ..Finding::default() }
    }

    #[test]
    fn findings_at_or_above_the_threshold_fail() {
        let found =
            [finding(Severity::Low), finding(Severity::High), finding(Severity::Critical), finding(Severity::High)];
        assert_eq!(findings_verdict(FailOn::High, Some(&found)).as_deref(), Some("1 critical, 2 high findings"));
        assert_eq!(findings_verdict(FailOn::Critical, Some(&found)).as_deref(), Some("1 critical finding"));
        assert_eq!(findings_verdict(FailOn::Never, Some(&found)), None);
        assert_eq!(findings_verdict(FailOn::High, Some(&[finding(Severity::Medium)])), None);
        assert_eq!(findings_verdict(FailOn::Info, None), None);
    }

    #[test]
    fn metric_checks_name_metrics() {
        let metrics = [Metric::new("p95", "P95", 640.0, "ms")];
        let check = |name: &str, value: &str| Assertion {
            expression: name.into(),
            operator: "lt".into(),
            value: Some(value.into()),
            disabled: false,
        };
        assert_eq!(metrics_verdict(&[check("p95", "1000")], &metrics), None);
        assert_eq!(
            metrics_verdict(&[check("p95", "500")], &metrics).as_deref(),
            Some("p95: expected lt 500, got 640.0")
        );
        assert!(metrics_verdict(&[check("p99", "500")], &metrics).is_some());
    }

    #[test]
    fn a_metric_written_as_null_reads_back_as_nan() {
        let json = serde_json::to_string(&Metric::new("errors", "Errors", f64::NAN, "%")).unwrap();
        assert!(serde_json::from_str::<Metric>(&json).unwrap().value.is_nan(), "{json}");
    }

    #[test]
    fn fail_on_defaults_to_high() {
        assert_eq!(serde_json::from_str::<FailOn>("\"never\"").unwrap(), FailOn::Never);
        assert!(FailOn::default().is_default());
        assert_eq!(FailOn::default(), FailOn::High);
    }
}
