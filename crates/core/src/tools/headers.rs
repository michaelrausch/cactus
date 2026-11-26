//! `- tool: headers`: send a request and check the security headers of its
//! response: HSTS, CSP, framing, MIME sniffing, referrer policy, version
//! disclosure, cookie flags and CORS. It sends the request once more with
//! a made-up `Origin` to see whether CORS trusts any site.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::findings::{Exports, export};
use super::{
    Coverage, FailOn, Finding, Severity, StatusCount, Tool, ToolCx, ToolOutput, findings_verdict, request_step,
    unchanged,
};
use crate::Result;
use crate::collection::{Header, LenientString};
use crate::http::Response;
use crate::scenario::Scenario;

/// The origin the CORS check claims to be sent from: one no API trusts.
pub const PROBE_ORIGIN: &str = "https://cactus-origin-check.example";
/// The shortest HSTS `max-age` that isn't reported: 180 days.
const MIN_HSTS_SECONDS: u64 = 15_552_000;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Headers {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub request: LenientString,
    /// Variables for this request only, as a request step's `with`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub with: BTreeMap<String, LenientString>,
    #[serde(skip_serializing_if = "FailOn::is_default")]
    pub fail_on: FailOn,
    /// Findings for later steps: `issues: findings`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub set: BTreeMap<String, LenientString>,
}

/// What `set` can name.
const EXPORTS: Exports = Exports { metrics: &[], findings: true };

impl Tool for Headers {
    fn title(&self) -> String {
        format!("Headers {}", self.request.trim())
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        EXPORTS.check(&self.set)?;
        let step = request_step(&self.request, &self.with);

        let response = cx.runner.probe(&step, &unchanged).await?;
        // Without an answer to this one, CORS just isn't checked.
        let cross_origin = cx.runner.probe(&step, &from_probe_origin).await.ok();

        let checked = check_all(&response, cross_origin.as_ref());
        let output = into_output(checked, response.status);

        export(cx, &self.set, &output)?;
        Ok(output)
    }

    fn verdict(&self, output: &ToolOutput) -> Option<String> {
        findings_verdict(self.fail_on, output.findings.as_deref())
    }
}

/// The request as a browser on another site would send it.
fn from_probe_origin(scenario: &mut Scenario) {
    let headers = &mut scenario.http.headers;
    headers.retain(|header| !header.name.trim().eq_ignore_ascii_case("origin"));
    headers.push(Header::new("Origin", PROBE_ORIGIN));
}

/// One header or cookie looked at, and what was wrong with it.
#[derive(Debug, Default)]
struct Checked {
    location: String,
    findings: Vec<Finding>,
}

impl Checked {
    fn new(location: impl Into<String>) -> Self {
        Self { location: location.into(), findings: Vec::new() }
    }

    fn report(&mut self, severity: Severity, title: &str, rule: &str, detail: impl Into<String>) {
        self.findings.push(Finding {
            severity,
            title: title.into(),
            rule: rule.into(),
            location: self.location.clone(),
            detail: detail.into(),
            ..Finding::default()
        });
    }
}

/// Every check that applies to `response`. `cross_origin` is the same
/// request sent from [`PROBE_ORIGIN`].
fn check_all(response: &Response, cross_origin: Option<&Response>) -> Vec<Checked> {
    let https = response.url.starts_with("https://");
    let html = is_html(response);

    let mut checked = Vec::new();
    if https {
        checked.push(hsts(response));
    }
    checked.push(csp(response, html));
    if html {
        checked.push(framing(response));
    }
    checked.push(nosniff(response));
    checked.push(referrer_policy(response));
    checked.push(server(response));
    checked.push(powered_by(response));
    checked.push(cors(response, cross_origin));
    checked.extend(cookies(response, https));

    checked
}

/// Findings most severe first, and a coverage row per thing checked.
fn into_output(checked: Vec<Checked>, status: u16) -> ToolOutput {
    let total = checked.len();
    let passed = checked.iter().filter(|checked| checked.findings.is_empty()).count();

    let coverage = checked
        .iter()
        .map(|checked| Coverage {
            location: checked.location.clone(),
            requests: 1,
            statuses: vec![StatusCount { status, count: 1 }],
            findings: u32::try_from(checked.findings.len()).unwrap_or(u32::MAX),
        })
        .collect();
    let mut findings: Vec<Finding> = checked.into_iter().flat_map(|checked| checked.findings).collect();
    findings.sort_by_key(|finding| std::cmp::Reverse(finding.severity));

    let text = format!("{passed} of {total} checks passed");
    ToolOutput { findings: Some(findings), coverage, status: Some(status), ..ToolOutput::text(text) }
}

/// Strict-Transport-Security: there, and for long enough.
fn hsts(response: &Response) -> Checked {
    let mut checked = Checked::new("Strict-Transport-Security");

    let Some(value) = response.header("strict-transport-security") else {
        checked.report(Severity::Medium, "Missing HSTS", "headers.hsts", "Browsers may connect over plain HTTP");
        return checked;
    };

    let max_age = directive(value, "max-age").and_then(|age| age.trim_matches('"').parse::<u64>().ok());
    if max_age.is_none_or(|age| age < MIN_HSTS_SECONDS) {
        let detail = format!("{value}; max-age should be at least {MIN_HSTS_SECONDS} (180 days)");
        checked.report(Severity::Low, "Short HSTS max-age", "headers.hsts-short", detail);
    }

    checked
}

/// Content-Security-Policy: there for HTML, and without unsafe scripts.
fn csp(response: &Response, html: bool) -> Checked {
    let mut checked = Checked::new("Content-Security-Policy");

    let Some(policy) = response.header("content-security-policy") else {
        // An API's JSON isn't rendered, so a CSP matters far less there.
        let severity = if html { Severity::Medium } else { Severity::Info };
        checked.report(severity, "Missing CSP", "headers.csp", "Nothing limits what scripts the page may run");
        return checked;
    };

    let scripts = directive(policy, "script-src").or_else(|| directive(policy, "default-src")).unwrap_or_default();
    let unsafe_sources: Vec<&str> =
        ["'unsafe-inline'", "'unsafe-eval'"].into_iter().filter(|source| scripts.contains(source)).collect();
    if !unsafe_sources.is_empty() {
        let detail = format!("Scripts allow {}", unsafe_sources.join(" and "));
        checked.report(Severity::Low, "CSP allows unsafe scripts", "headers.csp-unsafe", detail);
    }

    checked
}

/// X-Frame-Options or CSP `frame-ancestors`, so other sites can't frame it.
fn framing(response: &Response) -> Checked {
    let mut checked = Checked::new("X-Frame-Options");

    let frame_options = response.header("x-frame-options").is_some();
    let frame_ancestors = response.header("content-security-policy").and_then(|csp| directive(csp, "frame-ancestors"));
    if !frame_options && frame_ancestors.is_none() {
        let detail = "No X-Frame-Options or CSP frame-ancestors: other sites can frame the page";
        checked.report(Severity::Medium, "Can be framed", "headers.clickjacking", detail);
    }

    checked
}

/// X-Content-Type-Options: nosniff.
fn nosniff(response: &Response) -> Checked {
    let mut checked = Checked::new("X-Content-Type-Options");

    let value = response.header("x-content-type-options").unwrap_or_default();
    if !value.trim().eq_ignore_ascii_case("nosniff") {
        let detail = "Browsers may guess a response's type and run it as script";
        checked.report(Severity::Low, "MIME sniffing allowed", "headers.nosniff", detail);
    }

    checked
}

/// Referrer-Policy: there, and not sending full URLs everywhere.
fn referrer_policy(response: &Response) -> Checked {
    let mut checked = Checked::new("Referrer-Policy");

    match response.header("referrer-policy").map(str::trim) {
        None => {
            let detail = "Browsers decide what URL other sites see";
            checked.report(Severity::Low, "Missing Referrer-Policy", "headers.referrer-policy", detail);
        }
        Some(policy) if policy.eq_ignore_ascii_case("unsafe-url") => {
            let detail = "unsafe-url sends the full URL, query included, to every site";
            checked.report(Severity::Low, "Referrer-Policy leaks URLs", "headers.referrer-policy", detail);
        }
        Some(_) => {}
    }

    checked
}

/// A Server header that says which version is running.
fn server(response: &Response) -> Checked {
    let mut checked = Checked::new("Server");

    if let Some(value) = response.header("server").filter(|value| value.chars().any(|c| c.is_ascii_digit())) {
        let detail = format!("Server: {value}");
        checked.report(Severity::Low, "Server version disclosed", "headers.server-version", detail);
    }

    checked
}

/// X-Powered-By and its kin, which name the framework.
fn powered_by(response: &Response) -> Checked {
    let mut checked = Checked::new("X-Powered-By");

    for name in ["x-powered-by", "x-aspnet-version", "x-aspnetmvc-version"] {
        if let Some(value) = response.header(name) {
            let detail = format!("{name}: {value}");
            checked.report(Severity::Low, "Framework disclosed", "headers.powered-by", detail);
        }
    }

    checked
}

/// Whether CORS lets any site read responses, worst with credentials.
fn cors(response: &Response, cross_origin: Option<&Response>) -> Checked {
    let mut checked = Checked::new("Access-Control-Allow-Origin");
    let credentials = |response: &Response| {
        response.header("access-control-allow-credentials").is_some_and(|value| value.trim() == "true")
    };

    if response.header("access-control-allow-origin").is_some_and(|origin| origin.trim() == "*")
        && credentials(response)
    {
        let detail = "Allow-Origin * with Allow-Credentials true";
        checked.report(Severity::High, "CORS allows any origin with credentials", "headers.cors-credentials", detail);
    }

    let Some(cross_origin) = cross_origin else { return checked };
    if cross_origin.header("access-control-allow-origin").is_some_and(|origin| origin.trim() == PROBE_ORIGIN) {
        let detail = format!("Sent from {PROBE_ORIGIN}, it was allowed back");
        if credentials(cross_origin) {
            checked.report(Severity::High, "CORS trusts any origin, with credentials", "headers.cors-reflect", detail);
        } else {
            checked.report(Severity::Medium, "CORS trusts any origin", "headers.cors-reflect", detail);
        }
    }

    checked
}

/// Each cookie set: `Secure` (over HTTPS), `HttpOnly` and `SameSite`.
fn cookies(response: &Response, https: bool) -> Vec<Checked> {
    response
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, value)| cookie(value, https))
        .collect()
}

fn cookie(set_cookie: &str, https: bool) -> Checked {
    let mut parts = set_cookie.split(';').map(str::trim);
    let name = parts.next().and_then(|pair| pair.split('=').next()).unwrap_or_default().trim();
    let attributes: Vec<String> = parts.map(str::to_ascii_lowercase).collect();
    let has = |attribute: &str| attributes.iter().any(|a| a == attribute || a.starts_with(&format!("{attribute}=")));

    let mut checked = Checked::new(format!("cookie.{name}"));

    if https && !has("secure") {
        checked.report(Severity::Medium, "Cookie without Secure", "headers.cookie-secure", "Sent over plain HTTP too");
    }

    if !has("httponly") {
        let detail = "Scripts on the page can read it";
        checked.report(Severity::Low, "Cookie without HttpOnly", "headers.cookie-httponly", detail);
    }

    if !has("samesite") {
        let detail = "Browsers decide whether other sites' requests send it";
        checked.report(Severity::Low, "Cookie without SameSite", "headers.cookie-samesite", detail);
    }

    checked
}

/// The value of `name` in a `;`-separated header like CSP or HSTS.
fn directive<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').map(str::trim).find_map(|part| {
        let (key, value) = part.split_once(['=', ' ']).unwrap_or((part, ""));
        key.eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

fn is_html(response: &Response) -> bool {
    response.header("content-type").is_some_and(|value| value.to_ascii_lowercase().contains("html"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(url: &str, headers: &[(&str, &str)]) -> Response {
        let headers = headers.iter().map(|(name, value)| ((*name).into(), (*value).into())).collect();
        Response { url: url.into(), status: 200, headers, ..Response::default() }
    }

    fn rules(checked: &Checked) -> Vec<&str> {
        checked.findings.iter().map(|finding| finding.rule.as_str()).collect()
    }

    const SECURE: &[(&str, &str)] = &[
        ("content-type", "text/html"),
        ("strict-transport-security", "max-age=31536000; includeSubDomains"),
        ("content-security-policy", "default-src 'self'; frame-ancestors 'none'"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        ("server", "nginx"),
        ("set-cookie", "sid=abc; Path=/; Secure; HttpOnly; SameSite=Lax"),
    ];

    #[test]
    fn a_well_configured_response_passes_every_check() {
        let checked = check_all(&response("https://x.test/", SECURE), None);
        let findings: Vec<&Finding> = checked.iter().flat_map(|checked| &checked.findings).collect();
        assert_eq!(findings, Vec::<&Finding>::new());

        let locations: Vec<&str> = checked.iter().map(|checked| checked.location.as_str()).collect();
        assert_eq!(
            locations,
            [
                "Strict-Transport-Security",
                "Content-Security-Policy",
                "X-Frame-Options",
                "X-Content-Type-Options",
                "Referrer-Policy",
                "Server",
                "X-Powered-By",
                "Access-Control-Allow-Origin",
                "cookie.sid",
            ]
        );
    }

    #[test]
    fn a_bare_response_misses_everything_that_applies() {
        let html = response("https://x.test/", &[("content-type", "text/html")]);
        let checked = check_all(&html, None);
        let all: Vec<&str> = checked.iter().flat_map(rules).collect();
        assert_eq!(
            all,
            ["headers.hsts", "headers.csp", "headers.clickjacking", "headers.nosniff", "headers.referrer-policy"]
        );

        // Over plain HTTP there's no HSTS to ask for, and JSON isn't framed.
        let json = response("http://x.test/", &[("content-type", "application/json")]);
        let locations: Vec<String> = check_all(&json, None).into_iter().map(|checked| checked.location).collect();
        assert!(!locations.iter().any(|l| l == "Strict-Transport-Security" || l == "X-Frame-Options"), "{locations:?}");
        assert_eq!(csp(&json, false).findings[0].severity, Severity::Info);
    }

    #[test]
    fn weak_values_are_reported() {
        let short = response("https://x.test/", &[("strict-transport-security", "max-age=300")]);
        assert_eq!(rules(&hsts(&short)), ["headers.hsts-short"]);

        let unsafe_csp = response("https://x.test/", &[("content-security-policy", "script-src 'self' 'unsafe-eval'")]);
        let checked = csp(&unsafe_csp, true);
        assert_eq!(rules(&checked), ["headers.csp-unsafe"]);
        assert_eq!(checked.findings[0].detail, "Scripts allow 'unsafe-eval'");

        let leaky = response("https://x.test/", &[("server", "Apache/2.4.1"), ("x-powered-by", "PHP/8.1")]);
        assert_eq!(rules(&server(&leaky)), ["headers.server-version"]);
        assert_eq!(rules(&powered_by(&leaky)), ["headers.powered-by"]);
        assert_eq!(
            rules(&referrer_policy(&response("", &[("referrer-policy", "unsafe-url")]))),
            ["headers.referrer-policy"]
        );
    }

    #[test]
    fn cookies_need_their_flags() {
        let checked = cookie("token=x; Path=/", true);
        assert_eq!(checked.location, "cookie.token");
        assert_eq!(rules(&checked), ["headers.cookie-secure", "headers.cookie-httponly", "headers.cookie-samesite"]);
        assert_eq!(
            rules(&cookie("token=x; HttpOnly; SameSite=Strict", false)),
            Vec::<&str>::new(),
            "no Secure over HTTP"
        );
    }

    #[test]
    fn cors_that_trusts_anyone_is_reported() {
        let wildcard =
            response("", &[("access-control-allow-origin", "*"), ("access-control-allow-credentials", "true")]);
        assert_eq!(rules(&cors(&wildcard, None)), ["headers.cors-credentials"]);

        let plain = response("", &[]);
        let reflected = response("", &[("access-control-allow-origin", PROBE_ORIGIN)]);
        let checked = cors(&plain, Some(&reflected));
        assert_eq!((rules(&checked), checked.findings[0].severity), (vec!["headers.cors-reflect"], Severity::Medium));

        let with_credentials = response(
            "",
            &[("access-control-allow-origin", PROBE_ORIGIN), ("access-control-allow-credentials", "true")],
        );
        assert_eq!(cors(&plain, Some(&with_credentials)).findings[0].severity, Severity::High);
        assert_eq!(rules(&cors(&plain, Some(&plain))), Vec::<&str>::new());
    }

    #[test]
    fn the_probe_replaces_any_origin_the_request_sends() {
        let mut scenario = Scenario::default();
        scenario.http.headers = vec![Header::new("origin", "https://mine.test"), Header::new("Accept", "*/*")];
        from_probe_origin(&mut scenario);
        let names: Vec<(&str, &str)> =
            scenario.http.headers.iter().map(|header| (header.name.as_str(), header.value.as_str())).collect();
        assert_eq!(names, [("Accept", "*/*"), ("Origin", PROBE_ORIGIN)]);
    }

    #[test]
    fn output_counts_what_passed() {
        let output = into_output(check_all(&response("https://x.test/", SECURE), None), 200);
        assert_eq!(output.text.as_deref(), Some("9 of 9 checks passed"));
        assert_eq!(output.coverage.len(), 9);
        assert_eq!(output.found(), []);
    }
}
