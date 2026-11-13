//! The response expressions assertions and actions select values with, as
//! Bruno writes them: `res.status`, `res.body.data[0].id`, `res.headers.etag`,
//! `res.headers['content-type']`, `res.responseTime`. An expression without
//! `res.` (`data.token`, `$.data.token`) reads the body.

use std::collections::BTreeMap;

use serde_json::Value;

use super::response::Response;
use crate::jsonpath;
use crate::text::plain_string;

const RES: &str = "res";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Subject {
    Status,
    ResponseTime,
    Header(String),
    /// A `$`-rooted JSONPath into the body.
    Body(String),
}

impl Subject {
    pub(crate) fn parse(expression: &str) -> Option<Self> {
        let expression = expression.trim();
        let Some(rest) = expression.strip_prefix(RES).filter(|rest| rest.starts_with(['.', '['])) else {
            return Self::bare_body_path(expression);
        };
        match rest {
            ".status" | ".getStatus()" => Some(Self::Status),
            ".responseTime" | ".getResponseTime()" => Some(Self::ResponseTime),
            _ => {
                if let Some(path) = rest.strip_prefix(".body") {
                    return Some(Self::Body(body_path(path)?));
                }
                let header = rest.strip_prefix(".headers")?;
                header_name(header).map(Self::Header)
            }
        }
    }

    fn bare_body_path(expression: &str) -> Option<Self> {
        if expression.is_empty() {
            return None;
        }
        if expression.starts_with('$') {
            return Some(Self::Body(normalise_quotes(expression)));
        }
        let separator = if expression.starts_with('[') { "" } else { "." };
        Some(Self::Body(format!("${separator}{}", normalise_quotes(expression))))
    }

    pub(crate) fn reads_body(&self) -> bool {
        matches!(self, Self::Body(_))
    }

    /// The selected value, or `None` when it isn't there.
    pub(crate) fn select(&self, response: &Response, body: Option<&Value>) -> Option<Value> {
        match self {
            Self::Status => Some(Value::from(response.status)),
            Self::ResponseTime => Some(Value::from(response.time_ms)),
            Self::Header(name) => response.header(name).map(|value| Value::String(value.to_string())),
            Self::Body(path) => jsonpath::select(body?, path),
        }
    }
}

/// Whether `expression` reads the response status, as status assertions do.
pub(crate) fn is_status_expression(expression: &str) -> bool {
    Subject::parse(expression) == Some(Subject::Status)
}

/// The value each expression selects from `response`, as plain text, or
/// `None` when it selects nothing. The body is parsed once; a truncated or
/// non-JSON body gives `None` for body paths.
pub fn select_values(response: &Response, expressions: &[String]) -> BTreeMap<String, Option<String>> {
    let subjects: Vec<_> = expressions.iter().map(|e| (e, Subject::parse(e))).collect();
    let reads_body = subjects.iter().any(|(_, s)| s.as_ref().is_some_and(Subject::reads_body));
    let body =
        (reads_body && !response.truncated).then(|| serde_json::from_str::<Value>(&response.body).ok()).flatten();
    subjects
        .into_iter()
        .map(|(expression, subject)| {
            let value = subject.and_then(|s| s.select(response, body.as_ref())).map(|v| plain_string(&v));
            (expression.clone(), value)
        })
        .collect()
}

/// `""` → `$`, `.a[0]` → `$.a[0]`, `['a b']` → `$["a b"]`.
fn body_path(rest: &str) -> Option<String> {
    if rest.is_empty() || rest.starts_with(['.', '[']) { Some(format!("${}", normalise_quotes(rest))) } else { None }
}

/// `.etag` or `['content-type']` / `["content-type"]`.
fn header_name(rest: &str) -> Option<String> {
    if let Some(name) = rest.strip_prefix('.') {
        return (!name.is_empty()).then(|| name.to_string());
    }
    let inner = rest.strip_prefix('[')?.strip_suffix(']')?;
    let unquoted = inner.strip_prefix(['\'', '"'])?.strip_suffix(['\'', '"'])?;
    Some(unquoted.to_string())
}

/// The JSONPath subset only reads `["key"]`.
fn normalise_quotes(path: &str) -> String {
    path.replace("['", "[\"").replace("']", "\"]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bruno_expressions() {
        let body = |p: &str| Some(Subject::Body(p.into()));
        assert_eq!(Subject::parse("res.status"), Some(Subject::Status));
        assert_eq!(Subject::parse(" res.responseTime "), Some(Subject::ResponseTime));
        assert_eq!(Subject::parse("res.headers.etag"), Some(Subject::Header("etag".into())));
        assert_eq!(Subject::parse("res.headers['content-type']"), Some(Subject::Header("content-type".into())));
        assert_eq!(Subject::parse("res.body"), body("$"));
        assert_eq!(Subject::parse("res.body.data[0].id"), body("$.data[0].id"));
        assert_eq!(Subject::parse("res.body['a b']"), body(r#"$["a b"]"#));
        assert_eq!(Subject::parse("data.token"), body("$.data.token"));
        assert_eq!(Subject::parse("$.token"), body("$.token"));
        assert_eq!(Subject::parse("res.bodyx"), None);
        assert_eq!(Subject::parse("res.cookies"), None);
        assert_eq!(Subject::parse(""), None);
    }

    #[test]
    fn selects_watched_values() {
        let mut response: Response = serde_json::from_value(serde_json::json!({
            "method": "GET", "url": "", "status": 200, "statusText": "OK", "headers": [["etag", "x1"]],
            "body": r#"{"version": "1.4.2", "n": 3}"#, "truncated": false, "timeMs": 5, "sizeBytes": 0, "unresolved": []
        }))
        .unwrap();
        let expressions: Vec<String> =
            ["res.body.version", "res.body.n", "res.status", "res.headers.etag", "res.body.gone", "res.cookies"]
                .map(String::from)
                .into();
        let values = select_values(&response, &expressions);
        let got: Vec<_> = expressions.iter().map(|e| values[e].as_deref()).collect();
        assert_eq!(got, [Some("1.4.2"), Some("3"), Some("200"), Some("x1"), None, None]);
        response.truncated = true;
        assert_eq!(select_values(&response, &expressions)["res.body.version"], None);
        assert!(is_status_expression(" res.status") && !is_status_expression("res.body.status"));
    }
}
