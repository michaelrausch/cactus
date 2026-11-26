import { describe, expect, it } from "vitest";
import type { Finding } from "@/api/client";
import { countBySeverity, failsAt, findingsSummary, formatMetric, severityTone } from "./findings";

const finding = (severity: Finding["severity"], rule: string): Finding => ({ severity, title: rule, rule });

describe("findings", () => {
  const found = [finding("low", "a"), finding("critical", "b"), finding("low", "c"), finding("high", "d")];

  it("counts and summarises by severity", () => {
    expect(countBySeverity(found)).toEqual([
      ["critical", 1],
      ["high", 1],
      ["low", 2],
    ]);
    expect(findingsSummary(found)).toBe("1 critical · 1 high · 2 low");
    expect(findingsSummary([])).toBe("None found");
  });

  it("tones severities", () => {
    expect(["critical", "high", "medium", "low", "info"].map((s) => severityTone(s as Finding["severity"]))).toEqual([
      "danger",
      "danger",
      "warning",
      "info",
      "muted",
    ]);
  });

  it("says which severities fail a step", () => {
    expect(failsAt("high", undefined)).toBe(true);
    expect(failsAt("medium", undefined)).toBe(false);
    expect(failsAt("medium", "medium")).toBe(true);
    expect(failsAt("critical", "never")).toBe(false);
    expect(failsAt("info", "info")).toBe(true);
    expect(failsAt("high", "critical")).toBe(false);
  });

  it("formats metrics by unit", () => {
    expect(formatMetric({ name: "p95", label: "P95", value: 412.4, unit: "ms" })).toBe("412 ms");
    expect(formatMetric({ name: "errors", label: "Errors", value: 2.5, unit: "%" })).toBe("2.5%");
    expect(formatMetric({ name: "requests", label: "Requests", value: 1500 })).toBe("1500");
  });
});
