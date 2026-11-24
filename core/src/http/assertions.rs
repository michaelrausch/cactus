//! A request's assertions and set-variable actions, run against each response.

use std::collections::BTreeMap;

use regex::Regex;
use serde_json::Value;

use super::expression::Subject;
use super::response::{CaptureFailure, Check, Response};
use crate::scenario::{Action, ActionPhase, Assertion, Scenario, SetVariable};
use crate::text::plain_string;

/// Every operator [`holds`] checks, as Bruno names them. The editor offers
/// the same list (`ui/src/domain/http.ts`; `tests/ui_contract.rs` checks it).
pub const ASSERTION_OPERATORS: [&str; 28] = [
    "eq",
    "neq",
    "gt",
    "gte",
    "lt",
    "lte",
    "in",
    "notIn",
    "contains",
    "notContains",
    "startsWith",
    "endsWith",
    "matches",
    "notMatches",
    "length",
    "isDefined",
    "isUndefined",
    "isNull",
    "isNotNull",
    "isEmpty",
    "isNotEmpty",
    "isString",
    "isNumber",
    "isBoolean",
    "isArray",
    "isJson",
    "isTruthy",
    "isFalsy",
];

/// Without a status assertion, statuses from here up count as failures.
const FIRST_ERROR_STATUS: u16 = 400;

/// Fill in `captured` and `checks`. `raw_body` is the full body, which may
/// be longer than the truncated `response.body`.
pub(crate) fn evaluate(scenario: &Scenario, response: &mut Response, raw_body: &[u8]) {
    let body = needs_body(scenario).then(|| serde_json::from_slice::<Value>(raw_body).ok()).flatten();
    let (captured, failures) = captures(scenario, response, body.as_ref());
    response.captured = captured;
    response.capture_failures = failures;
    response.checks = active_assertions(scenario).map(|a| check(a, response, body.as_ref())).collect();
}

/// Whether the request passes: every check holds and, unless the status is
/// asserted explicitly, the status is not an error.
pub fn scenario_passed(scenario: &Scenario, response: &Response) -> bool {
    let all_checks_ok = response.checks.iter().all(|c| c.ok);
    let checks_status = active_assertions(scenario).any(|a| Subject::parse(&a.expression) == Some(Subject::Status));
    all_checks_ok && (checks_status || response.status < FIRST_ERROR_STATUS)
}

fn active_assertions(scenario: &Scenario) -> impl Iterator<Item = &Assertion> {
    scenario.runtime.assertions.iter().filter(|a| !a.disabled && !a.expression.trim().is_empty())
}

/// The set-variable actions run after the response; `before-request` ones
/// have nothing to read yet.
pub(crate) fn response_actions(scenario: &Scenario) -> impl Iterator<Item = &SetVariable> {
    scenario.runtime.actions.iter().filter_map(|action| match action {
        Action::SetVariable(set)
            if !set.disabled
                && set.phase != Some(ActionPhase::BeforeRequest)
                && !set.variable.name.trim().is_empty() =>
        {
            Some(set)
        }
        _ => None,
    })
}

/// Parsing large bodies is skipped when nothing reads them.
fn needs_body(scenario: &Scenario) -> bool {
    let reads_body = |expression: &str| Subject::parse(expression).is_some_and(|s| s.reads_body());
    active_assertions(scenario).any(|a| reads_body(&a.expression))
        || response_actions(scenario).any(|set| reads_body(&set.selector.expression))
}

/// The values the response's set-variable actions select, and the actions
/// that found nothing.
fn captures(
    scenario: &Scenario,
    response: &Response,
    body: Option<&Value>,
) -> (BTreeMap<String, String>, Vec<CaptureFailure>) {
    let mut captured = BTreeMap::new();
    let mut failures = Vec::new();
    for set in response_actions(scenario) {
        let name = set.variable.name.trim().to_string();
        let expression = set.selector.expression.trim().to_string();
        match capture(&expression, response, body) {
            Ok(value) => {
                captured.insert(name, value);
            }
            Err(reason) => failures.push(CaptureFailure { name, expression, reason }),
        }
    }
    (captured, failures)
}

fn capture(expression: &str, response: &Response, body: Option<&Value>) -> Result<String, String> {
    if expression.is_empty() {
        return Err("no expression".into());
    }
    let subject = Subject::parse(expression).ok_or_else(|| "unsupported expression".to_string())?;
    if subject.reads_body() && body.is_none() {
        return Err("response body isn't JSON".into());
    }
    match subject.select(response, body) {
        Some(value) => Ok(plain_string(&value)),
        None => Err(format!("nothing at {expression}")),
    }
}

fn check(assertion: &Assertion, response: &Response, body: Option<&Value>) -> Check {
    let name = assertion.expression.trim().to_string();
    let operator = assertion.operator.trim();
    let expected_text = assertion.value.as_ref().map_or("", |value| value.trim());
    let wanted = if expected_text.is_empty() { operator.to_string() } else { format!("{operator} {expected_text}") };
    let Some(subject) = Subject::parse(&name) else {
        return Check { name, ok: false, detail: "unsupported expression".into() };
    };
    let actual = subject.select(response, body);
    let got = actual.as_ref().map_or_else(|| "nothing".into(), Value::to_string);
    match holds(operator, actual.as_ref(), expected_text) {
        Ok(ok) => Check { name, ok, detail: format!("expected {wanted}, got {got}") },
        Err(problem) => Check { name, ok: false, detail: problem },
    }
}

/// Whether `actual operator expected` holds, or why it can't be checked:
/// an unknown operator or an invalid pattern.
fn holds(operator: &str, actual: Option<&Value>, expected: &str) -> Result<bool, String> {
    let value = || parse_expected(expected);
    let number = |v: Option<&Value>| v.and_then(as_number);
    let compare = |test: fn(f64, f64) -> bool| match (number(actual), as_number(&value())) {
        (Some(a), Some(e)) => test(a, e),
        _ => false,
    };
    let text = || actual.map_or_default(plain_string);
    let expected_text = || plain_string(&value());
    Ok(match operator {
        "eq" => actual.is_some_and(|a| loosely_equal(a, &value())),
        "neq" => !actual.is_some_and(|a| loosely_equal(a, &value())),
        "gt" => compare(|a, e| a > e),
        "gte" => compare(|a, e| a >= e),
        "lt" => compare(|a, e| a < e),
        "lte" => compare(|a, e| a <= e),
        "in" => actual.is_some_and(|a| expected_list(expected).iter().any(|e| loosely_equal(a, e))),
        "notIn" => !actual.is_some_and(|a| expected_list(expected).iter().any(|e| loosely_equal(a, e))),
        "contains" => actual.is_some_and(|a| contains(a, &value())),
        "notContains" => !actual.is_some_and(|a| contains(a, &value())),
        "startsWith" => actual.is_some() && text().starts_with(&expected_text()),
        "endsWith" => actual.is_some() && text().ends_with(&expected_text()),
        "matches" => actual.is_some() && pattern(expected)?.is_match(&text()),
        "notMatches" => !(actual.is_some() && pattern(expected)?.is_match(&text())),
        "length" => actual.and_then(length).is_some_and(|len| as_number(&value()) == Some(len)),
        "isDefined" => actual.is_some(),
        "isUndefined" => actual.is_none(),
        "isNull" => actual == Some(&Value::Null),
        "isNotNull" => actual.is_some_and(|a| !a.is_null()),
        "isEmpty" => actual.and_then(length) == Some(0.0),
        "isNotEmpty" => actual.and_then(length).is_some_and(|len| len > 0.0),
        "isString" => actual.is_some_and(Value::is_string),
        "isNumber" => actual.is_some_and(Value::is_number),
        "isBoolean" => actual.is_some_and(Value::is_boolean),
        "isArray" => actual.is_some_and(Value::is_array),
        "isJson" => actual.is_some_and(|a| a.is_object() || a.is_array()),
        "isTruthy" => actual.is_some_and(truthy),
        "isFalsy" => !actual.is_some_and(truthy),
        _ => return Err(format!("unsupported operator {operator:?}")),
    })
}

/// `200` and `true` are JSON, `"ok"` is a quoted string, anything else is
/// the text as written.
fn parse_expected(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()))
}

/// A JSON array, or comma-separated values.
fn expected_list(text: &str) -> Vec<Value> {
    match parse_expected(text) {
        Value::Array(items) => items,
        _ => text.split(',').map(|item| parse_expected(item.trim())).collect(),
    }
}

/// Equal, with `200` and `200.0` the same number.
fn loosely_equal(actual: &Value, expected: &Value) -> bool {
    match (as_number(actual), as_number(expected)) {
        (Some(a), Some(e)) if actual.is_number() && expected.is_number() => (a - e).abs() < f64::EPSILON,
        _ => actual == expected,
    }
}

/// A number, or text that reads as one.
fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn contains(actual: &Value, expected: &Value) -> bool {
    match actual {
        Value::Array(items) => items.iter().any(|item| loosely_equal(item, expected)),
        Value::Object(fields) => expected.as_str().is_some_and(|key| fields.contains_key(key)),
        other => plain_string(other).contains(&plain_string(expected)),
    }
}

#[allow(clippy::cast_precision_loss)]
fn length(value: &Value) -> Option<f64> {
    Some(match value {
        Value::String(s) => s.chars().count() as f64,
        Value::Array(items) => items.len() as f64,
        Value::Object(fields) => fields.len() as f64,
        _ => return None,
    })
}

/// JavaScript truthiness.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|n| n != 0.0 && !n.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// A regex, optionally written `/like this/`. An invalid one can't be checked.
fn pattern(text: &str) -> Result<Regex, String> {
    let text = text.trim();
    let source = text.strip_prefix('/').and_then(|t| t.strip_suffix('/')).unwrap_or(text);
    Regex::new(source).map_err(|_| format!("invalid pattern {text:?}"))
}

#[cfg(test)]
mod tests {
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
            time_ms: 12,
            size_bytes: 0,
            unresolved: vec![],
            warnings: vec![],
            captured: BTreeMap::new(),
            capture_failures: vec![],
            checks: vec![],
        }
    }

    fn assertion(expression: &str, operator: &str, value: &str) -> Assertion {
        let value = (!value.is_empty()).then(|| value.into());
        Assertion { expression: expression.into(), operator: operator.into(), value, disabled: false }
    }

    #[test]
    fn evaluate_actions_and_assertions() {
        let mut s = Scenario::default();
        s.runtime.actions = vec![
            Action::SetVariable(SetVariable::new("token", "res.body.auth.token")),
            Action::SetVariable(SetVariable::new("loc", "res.headers.Location")),
            Action::SetVariable(SetVariable::new("none", "res.body.nope")),
        ];
        s.runtime.assertions = vec![
            assertion("res.status", "in", "200, 201"),
            assertion("res.body.ok", "eq", "true"),
            assertion("res.body.n", "eq", "2"),
            assertion("res.body.n", "frobs", "2"),
            Assertion { disabled: true, ..assertion("res.status", "eq", "500") },
        ];
        let raw = br#"{"auth":{"token":"T"},"ok":true,"n":1}"#;
        let mut res = response(201);
        evaluate(&s, &mut res, raw);
        assert_eq!(res.captured, BTreeMap::from([("loc".into(), "/x".into()), ("token".into(), "T".into())]));
        let failure = CaptureFailure {
            name: "none".into(),
            expression: "res.body.nope".into(),
            reason: "nothing at res.body.nope".into(),
        };
        assert_eq!(res.capture_failures, [failure]);
        let oks: Vec<_> = res.checks.iter().map(|c| (c.name.as_str(), c.ok)).collect();
        assert_eq!(oks, [("res.status", true), ("res.body.ok", true), ("res.body.n", false), ("res.body.n", false)]);
        assert_eq!(res.checks[0].detail, "expected in 200, 201, got 201");
        assert_eq!(res.checks[3].detail, r#"unsupported operator "frobs""#);
    }

    #[test]
    fn capture_from_non_json_body_fails() {
        let mut s = Scenario::default();
        s.runtime.actions = vec![
            Action::SetVariable(SetVariable::new("token", "res.body.token")),
            Action::SetVariable(SetVariable::new("etag", "res.headers.etag")),
        ];
        let mut res = response(200);
        evaluate(&s, &mut res, b"<html>");
        assert!(res.captured.is_empty());
        let reasons: Vec<_> = res.capture_failures.iter().map(|f| (f.name.as_str(), f.reason.as_str())).collect();
        assert_eq!(reasons, [("token", "response body isn't JSON"), ("etag", "nothing at res.headers.etag")]);
    }

    #[test]
    fn operators() {
        let v = serde_json::json!({"s": "Hello", "n": 5, "list": [1, "a"], "empty": [], "obj": {"k": 1}, "nil": null});
        let at = |path: &str| crate::jsonpath::get(&v, path);
        let ok = |op: &str, path: &str, expected: &str| holds(op, at(path), expected).unwrap();
        assert!(ok("eq", "$.s", "Hello") && ok("eq", "$.s", "\"Hello\"") && ok("eq", "$.n", "5.0"));
        assert!(!ok("eq", "$.n", "\"5\"") && ok("neq", "$.n", "6") && ok("neq", "$.missing", "1"));
        assert!(ok("gt", "$.n", "4") && ok("lte", "$.n", "5") && !ok("lt", "$.s", "3"));
        assert!(ok("in", "$.n", "[4, 5]") && ok("notIn", "$.n", "1,2"));
        assert!(ok("contains", "$.s", "ell") && ok("contains", "$.list", "a") && ok("contains", "$.obj", "k"));
        assert!(ok("startsWith", "$.s", "He") && ok("endsWith", "$.s", "lo") && ok("matches", "$.s", "/^H.l+o$/"));
        assert!(ok("length", "$.list", "2") && ok("isEmpty", "$.empty", "") && ok("isNotEmpty", "$.s", ""));
        assert!(ok("isDefined", "$.nil", "") && ok("isUndefined", "$.missing", "") && ok("isNull", "$.nil", ""));
        assert!(ok("isString", "$.s", "") && ok("isNumber", "$.n", "") && ok("isArray", "$.list", ""));
        assert!(ok("isJson", "$.obj", "") && ok("isTruthy", "$.n", "") && ok("isFalsy", "$.nil", ""));
        assert_eq!(holds("matches", at("$.s"), "("), Err(r#"invalid pattern "(""#.into()));
    }

    #[test]
    fn every_listed_operator_is_checked() {
        for operator in ASSERTION_OPERATORS {
            assert!(holds(operator, Some(&Value::from("x")), "x").is_ok(), "{operator}");
        }
        assert!(holds("frobs", None, "").is_err());
    }

    #[test]
    fn passing_rules() {
        let plain = Scenario::default();
        assert!(scenario_passed(&plain, &response(399)));
        assert!(!scenario_passed(&plain, &response(404)));
        let mut expects_404 = Scenario::default();
        expects_404.runtime.assertions.push(assertion("res.status", "eq", "404"));
        let mut res = response(404);
        evaluate(&expects_404, &mut res, b"");
        assert!(scenario_passed(&expects_404, &res));
    }
}
