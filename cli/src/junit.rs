//! `JUnit` XML for flow runs, which CI systems show as test reports: one
//! `<testsuite>` per critter, one `<testcase>` per step.

use std::fmt::Write;

use cactus_core::runner::{FlowRun, FlowStepResult};

/// `runs` as `(critter, flow, run)`, in the order they ran.
// The `let _ = write!` calls below write to a `String`, which can't fail.
pub fn report(runs: &[(String, String, FlowRun)]) -> String {
    let mut critters: Vec<&str> = Vec::new();
    for (critter, _, _) in runs {
        if !critters.contains(&critter.as_str()) {
            critters.push(critter);
        }
    }
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites>\n");
    for critter in critters {
        let steps: Vec<(&str, &FlowStepResult)> = runs
            .iter()
            .filter(|(c, _, _)| c == critter)
            .flat_map(|(_, flow, run)| run.steps.iter().map(move |step| (flow.as_str(), step)))
            .collect();
        let failures = steps.iter().filter(|(_, s)| !s.ok && !s.skipped).count();
        let skipped = steps.iter().filter(|(_, s)| s.skipped).count();
        let _ = writeln!(
            xml,
            "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{failures}\" skipped=\"{skipped}\">",
            escape(critter),
            steps.len()
        );
        for (flow, step) in steps {
            write_case(&mut xml, critter, flow, step);
        }
        xml.push_str("  </testsuite>\n");
    }
    xml.push_str("</testsuites>\n");
    xml
}

fn write_case(xml: &mut String, critter: &str, flow: &str, step: &FlowStepResult) {
    let name = format!("{flow} › {}. {}", step.index + 1, step.label());
    let seconds = step.response.as_ref().map_or(0.0, |r| seconds(r.time_ms));
    let _ =
        write!(xml, "    <testcase name=\"{}\" classname=\"{}\" time=\"{seconds:.3}\"", escape(&name), escape(critter));
    if step.skipped {
        xml.push_str(">\n      <skipped/>\n    </testcase>\n");
    } else if step.ok {
        xml.push_str("/>\n");
    } else {
        let _ = writeln!(
            xml,
            ">\n      <failure message=\"{}\">{}</failure>\n    </testcase>",
            escape(&step.reason()),
            escape(&failure_details(step))
        );
    }
}

/// Every failed check, or the error, one per line.
fn failure_details(step: &FlowStepResult) -> String {
    if let Some(error) = &step.error {
        return error.clone();
    }
    let Some(response) = &step.response else { return String::new() };
    let failed: Vec<String> =
        response.checks.iter().filter(|c| !c.ok).map(|c| format!("{}: {}", c.name, c.detail)).collect();
    if failed.is_empty() { format!("status {}", response.status) } else { failed.join("\n") }
}

// Response times stay far below 2^52 ms, so nothing is lost.
#[allow(clippy::cast_precision_loss)]
fn seconds(millis: u64) -> f64 {
    millis as f64 / 1000.0
}

/// Escape text for an XML attribute or element. Characters XML 1.0 can't
/// hold at all are dropped.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if u32::from(c) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cactus_core::http::{Check, Response};

    use super::*;

    fn response(status: u16, checks: &[Check]) -> Response {
        serde_json::from_value(serde_json::json!({
            "method": "GET", "url": "", "status": status, "statusText": "", "headers": [], "body": "",
            "truncated": false, "timeMs": 1250, "sizeBytes": 0, "unresolved": [], "checks": checks
        }))
        .unwrap()
    }

    fn step(index: u32, request: &str, ok: bool, skipped: bool, response: Option<Response>) -> FlowStepResult {
        FlowStepResult {
            index,
            request: request.into(),
            tool: None,
            output: None,
            title: request.into(),
            method: "GET".into(),
            url: "http://h/".into(),
            ok,
            skipped,
            response,
            error: None,
            watched: BTreeMap::new(),
        }
    }

    #[test]
    fn writes_one_suite_per_critter_and_escapes() {
        let bad_check =
            Check { name: "res.body.role".into(), ok: false, detail: "expected eq \"<admin>\" & more".into() };
        let bob = FlowRun {
            at: 0,
            env: None,
            ok: false,
            elapsed_ms: 0,
            steps: vec![
                step(0, "auth/login.yml", true, false, Some(response(200, &[]))),
                step(1, "users/<1>.yml", false, false, Some(response(200, &[bad_check]))),
                step(2, "me.yml", false, true, None),
            ],
        };
        let alice = FlowRun { ok: true, steps: vec![step(0, "me.yml", true, false, None)], ..bob.clone() };
        let runs =
            vec![("bob".to_string(), "health".to_string(), bob), ("alice".to_string(), "smoke".to_string(), alice)];
        assert_eq!(
            report(&runs),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="bob" tests="3" failures="1" skipped="1">
    <testcase name="health › 1. auth/login.yml" classname="bob" time="1.250"/>
    <testcase name="health › 2. users/&lt;1&gt;.yml" classname="bob" time="1.250">
      <failure message="res.body.role: expected eq &quot;&lt;admin&gt;&quot; &amp; more">res.body.role: expected eq &quot;&lt;admin&gt;&quot; &amp; more</failure>
    </testcase>
    <testcase name="health › 3. me.yml" classname="bob" time="0.000">
      <skipped/>
    </testcase>
  </testsuite>
  <testsuite name="alice" tests="1" failures="0" skipped="0">
    <testcase name="smoke › 1. me.yml" classname="alice" time="0.000"/>
  </testsuite>
</testsuites>
"#
        );
        assert_eq!(escape("a\u{1}b'"), "ab&apos;");
    }
}
