//! `capture` rules and `expect` checks, run against each response.

use std::collections::BTreeMap;

use serde_json::Value;

use super::response::{Check, Response};
use crate::jsonpath;
use crate::scenario::{Expect, Scenario, StatusExpect};
use crate::text::plain_string;

const HEADER_CAPTURE_PREFIX: &str = "header:";
const STATUS_CHECK_NAME: &str = "status";
/// Without a status expectation, statuses from here up count as failures.
const FIRST_ERROR_STATUS: u16 = 400;

/// Where a capture rule reads its value from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureSource<'a> {
    Header(&'a str),
    JsonPath(&'a str),
}

impl<'a> CaptureSource<'a> {
    fn parse(rule: &'a str) -> Self {
        match rule.strip_prefix(HEADER_CAPTURE_PREFIX) {
            Some(name) => Self::Header(name.trim()),
            None => Self::JsonPath(rule),
        }
    }
}

/// Fill in `captured` and `checks`. `raw_body` is the full body, which may
/// be longer than the truncated `response.body`.
pub(crate) fn evaluate(scenario: &Scenario, response: &mut Response, raw_body: &[u8]) {
    let json = needs_json_body(scenario).then(|| serde_json::from_slice::<Value>(raw_body).ok()).flatten();
    response.captured = apply_captures(&scenario.capture, response, json.as_ref());
    if let Some(expect) = &scenario.expect {
        response.checks = expectation_checks(expect, response.status, json.as_ref());
    }
}

/// Whether the scenario passes: every check holds and, unless the status is
/// checked explicitly, the status is not an error.
pub fn scenario_passed(scenario: &Scenario, response: &Response) -> bool {
    let all_checks_ok = response.checks.iter().all(|c| c.ok);
    all_checks_ok && (scenario.has_status_expectation() || response.status < FIRST_ERROR_STATUS)
}

/// Parsing large bodies is skipped when no rule needs them.
fn needs_json_body(scenario: &Scenario) -> bool {
    let captures_json =
        scenario.capture.values().any(|rule| matches!(CaptureSource::parse(rule), CaptureSource::JsonPath(_)));
    let checks_json = scenario.expect.as_ref().is_some_and(|e| !e.json.is_empty());
    captures_json || checks_json
}

fn apply_captures(
    rules: &BTreeMap<String, String>,
    response: &Response,
    json: Option<&Value>,
) -> BTreeMap<String, String> {
    rules
        .iter()
        .filter_map(|(name, rule)| Some((name.clone(), capture_value(CaptureSource::parse(rule), response, json)?)))
        .collect()
}

fn capture_value(source: CaptureSource, response: &Response, json: Option<&Value>) -> Option<String> {
    match source {
        CaptureSource::Header(name) => response.header(name).map(str::to_string),
        CaptureSource::JsonPath(path) => jsonpath::get(json?, path).map(plain_string),
    }
}

fn expectation_checks(expect: &Expect, status: u16, json: Option<&Value>) -> Vec<Check> {
    let status_check = expect.status.as_ref().map(|expected| status_check(expected, status));
    let json_checks = expect.json.iter().map(|(path, expected)| json_check(path, expected, json));
    status_check.into_iter().chain(json_checks).collect()
}

fn status_check(expected: &StatusExpect, actual: u16) -> Check {
    Check {
        name: STATUS_CHECK_NAME.into(),
        ok: expected.accepts(actual),
        detail: format!("expected {}, got {actual}", expected.describe()),
    }
}

fn json_check(path: &str, expected: &Value, json: Option<&Value>) -> Check {
    let actual = json.and_then(|j| jsonpath::get(j, path));
    Check {
        name: path.to_string(),
        ok: actual == Some(expected),
        detail: format!("expected {expected}, got {}", actual.map_or_else(|| "nothing".into(), Value::to_string)),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn response(status: u16) -> Response {
        Response {
            method: "GET".into(),
            url: String::new(),
            status,
            status_text: String::new(),
            headers: vec![("location".into(), "/x".into())],
            body: String::new(),
            truncated: false,
            time_ms: 0,
            size_bytes: 0,
            unresolved: vec![],
            captured: BTreeMap::new(),
            checks: vec![],
        }
    }

    #[test]
    fn evaluate_captures_and_checks() {
        let s = Scenario {
            capture: BTreeMap::from([
                ("token".into(), "$.auth.token".into()),
                ("loc".into(), "header:Location".into()),
                ("none".into(), "$.nope".into()),
            ]),
            expect: Some(Expect {
                status: Some(StatusExpect::Any(vec![200, 201])),
                json: BTreeMap::from([("$.ok".into(), json!(true)), ("$.n".into(), json!(2))]),
            }),
            ..Default::default()
        };
        let raw = br#"{"auth":{"token":"T"},"ok":true,"n":1}"#;
        let mut res = response(201);
        evaluate(&s, &mut res, raw);
        assert_eq!(res.captured, BTreeMap::from([("loc".into(), "/x".into()), ("token".into(), "T".into())]));
        let oks: Vec<_> = res.checks.iter().map(|c| (c.name.as_str(), c.ok)).collect();
        assert_eq!(oks, [("status", true), ("$.n", false), ("$.ok", true)]);
        assert_eq!(res.checks[0].detail, "expected 200 | 201, got 201");
    }

    #[test]
    fn parses_capture_sources() {
        assert_eq!(CaptureSource::parse("header: Location"), CaptureSource::Header("Location"));
        assert_eq!(CaptureSource::parse("$.a"), CaptureSource::JsonPath("$.a"));
    }

    #[test]
    fn passing_rules() {
        let plain = Scenario::default();
        assert!(scenario_passed(&plain, &response(399)));
        assert!(!scenario_passed(&plain, &response(404)));
        let expects_404 = Scenario {
            expect: Some(Expect { status: Some(StatusExpect::One(404)), ..Default::default() }),
            ..Default::default()
        };
        let mut res = response(404);
        evaluate(&expects_404, &mut res, b"");
        assert!(scenario_passed(&expects_404, &res));
    }
}
