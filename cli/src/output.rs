//! Human-readable terminal output.

use cactus_core::http::Response;
use cactus_core::import::ImportSummary;
use cactus_core::runner::{FlowRun, FlowStepResult, RunResult};

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";

const PASS_MARK: &str = "✓";
const FAIL_MARK: &str = "✗";
const SKIP_MARK: &str = "-";
/// Lines under a result are indented past the mark and status columns.
const DETAIL_INDENT: &str = "      ";

fn paint(color: &str, text: &str) -> String {
    format!("{color}{text}{RESET}")
}

pub fn format_run_result(result: &RunResult) -> String {
    let location = format!("{}  {}", result.title, paint(DIM, result.rel_path.as_str()));
    format_outcome(result.ok, &location, result.response.as_ref(), result.error.as_deref())
}

/// One step of a flow; steps after a failure show as skipped.
pub fn format_step_result(step: &FlowStepResult) -> String {
    let location = format!("{}. {}  {}", step.index + 1, step.title, paint(DIM, &step.request));
    if step.skipped {
        return format!("{} {}", paint(DIM, SKIP_MARK), paint(DIM, &format!("  -        -  {location}  skipped")));
    }
    format_outcome(step.ok, &location, step.response.as_ref(), step.error.as_deref())
}

fn format_outcome(ok: bool, location: &str, response: Option<&Response>, error: Option<&str>) -> String {
    let mark = if ok { paint(GREEN, PASS_MARK) } else { paint(RED, FAIL_MARK) };
    if let Some(response) = response {
        let mut lines = vec![format!("{mark} {:>3} {:>6}ms  {location}", response.status, response.time_ms)];
        lines.extend(failed_check_lines(response));
        lines.extend(capture_failure_lines(response));
        lines.extend(unresolved_line(response));
        lines.extend(warning_lines(response));
        return lines.join("\n");
    }
    format!("{mark}   -        -  {location}\n{DETAIL_INDENT}{}", paint(RED, error.unwrap_or_default()))
}

/// The heading printed before a flow's steps.
pub fn format_flow_heading(critter: &str, flow: &str) -> String {
    format!("\n{critter} › {flow}")
}

/// How many flows passed, after every flow has run.
pub fn format_flow_summary(runs: &[(String, String, FlowRun)]) -> String {
    let passed = runs.iter().filter(|(_, _, run)| run.ok).count();
    let noun = if runs.len() == 1 { "flow" } else { "flows" };
    format!("\n{passed}/{} {noun} passed", runs.len())
}

fn failed_check_lines(response: &Response) -> impl Iterator<Item = String> + '_ {
    response
        .checks
        .iter()
        .filter(|check| !check.ok)
        .map(|check| format!("{DETAIL_INDENT}{}", paint(RED, &format!("{}: {}", check.name, check.detail))))
}

/// Set-variable actions that found nothing, one line each.
fn capture_failure_lines(response: &Response) -> impl Iterator<Item = String> + '_ {
    response.capture_failures.iter().map(|failure| {
        let text = format!("capture {} failed: {}", failure.name, failure.reason);
        format!("{DETAIL_INDENT}{}", paint(YELLOW, &text))
    })
}

fn unresolved_line(response: &Response) -> Option<String> {
    (!response.unresolved.is_empty()).then(|| {
        format!("{DETAIL_INDENT}{}", paint(YELLOW, &format!("unresolved: {}", response.unresolved.join(", "))))
    })
}

fn warning_lines(response: &Response) -> impl Iterator<Item = String> + '_ {
    response.warnings.iter().map(|warning| format!("{DETAIL_INDENT}{}", paint(YELLOW, warning)))
}

pub fn format_run_summary(results: &[RunResult]) -> String {
    let passed = results.iter().filter(|r| r.ok).count();
    format!("\n{passed}/{} passed", results.len())
}

/// The status line, meant for stderr so stdout carries only the body.
pub fn format_status_line(response: &Response) -> String {
    format!("{} {} · {}ms", response.status, response.status_text, response.time_ms)
}

pub fn format_import_summary(summary: &ImportSummary) -> String {
    let imported = format!("imported {} requests from {}", summary.created.len(), summary.format.as_str());
    if summary.env_vars.is_empty() {
        return imported;
    }
    format!("{imported}\nadded to opencollection.yml: {}", summary.env_vars.join(", "))
}

#[cfg(test)]
mod tests {
    use cactus_core::ProjectPath;

    use super::*;

    #[test]
    fn error_result_line() {
        let result = RunResult {
            rel_path: ProjectPath::parse("a.yml").unwrap(),
            title: "A".into(),
            ok: false,
            response: None,
            error: Some("boom".into()),
        };
        assert_eq!(
            format_run_result(&result),
            "\x1b[31m✗\x1b[0m   -        -  A  \x1b[2ma.yml\x1b[0m\n      \x1b[31mboom\x1b[0m"
        );
    }
}
