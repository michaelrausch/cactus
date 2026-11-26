import type { FailOn, Finding, Metric, Severity } from "@/api/client";
import { formatDuration } from "@/lib/format";

/** Most severe first. Mirrors `Severity`'s order in Rust. */
export const SEVERITIES: readonly Severity[] = ["critical", "high", "medium", "low", "info"];

/** How many of each severity, most severe first, leaving out none-found. */
export function countBySeverity(findings: readonly { severity?: Severity }[]): [Severity, number][] {
  return SEVERITIES.map((severity): [Severity, number] => [
    severity,
    findings.filter((finding) => (finding.severity ?? "info") === severity).length,
  ]).filter(([, count]) => count > 0);
}

/** `"2 high · 1 medium"`, or `"None found"`. */
export function findingsSummary(findings: readonly { severity?: Severity }[]): string {
  const counts = countBySeverity(findings);
  if (counts.length === 0) return "None found";
  return counts.map(([severity, count]) => `${count} ${severity}`).join(" · ");
}

/** The theme colour a severity is shown in. */
export function severityTone(severity: Severity | undefined): "danger" | "warning" | "info" | "muted" {
  switch (severity ?? "info") {
    case "critical":
    case "high":
      return "danger";
    case "medium":
      return "warning";
    case "low":
      return "info";
    case "info":
      return "muted";
  }
}

/** `412 ms`, `2.5%`, `120/s`, `1500`: milliseconds as a duration, the rest with one decimal at most. */
export function formatMetric(metric: Metric): string {
  // A non-finite number reaches the UI as null.
  if (metric.value === null) return "—";
  if (metric.unit === "ms") return formatDuration(Math.round(metric.value));
  const value = Number.isInteger(metric.value) ? String(metric.value) : metric.value.toFixed(1);
  return `${value}${metric.unit ?? ""}`;
}

/** What a security tool's `set` can name: its metrics, and its findings if it has any. Mirrors `Exports` in Rust. */
export function exportSources(metrics: readonly string[], findings: boolean): string[] {
  const found = findings ? ["findings", "findings.count", ...SEVERITIES.map((s) => `findings.${s}`)] : [];
  return [...metrics, ...found];
}

/** A problem for each `set` entry naming something the tool doesn't report. */
export function setProblems(set: Readonly<Record<string, string>> | undefined, sources: readonly string[]): string[] {
  return Object.entries(set ?? {})
    .filter(([, source]) => !sources.includes(source.trim()))
    .map(([name, source]) => `Set ${name}: no ${source.trim() || "value"}`);
}

/** Whether a finding of `severity` fails a step with `failOn` (default `high`). Mirrors `findings_verdict` in Rust. */
export function failsAt(severity: Severity, failOn: FailOn | undefined): boolean {
  const threshold = failOn ?? "high";
  if (threshold === "never") return false;
  // SEVERITIES is most severe first, so a lower index is worse.
  return SEVERITIES.indexOf(severity) <= SEVERITIES.indexOf(threshold);
}

/** A finding's title, or its rule when untitled. */
export const findingTitle = (finding: Finding): string => finding.title || finding.rule || "Finding";
