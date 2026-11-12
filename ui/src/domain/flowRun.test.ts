import { describe, expect, it } from "vitest";
import type { FlowRun, FlowStepResult, HttpResponse } from "@/api/client";
import { flowRunSummary, stepFailure } from "./flowRun";

const response = (status: number, checks: HttpResponse["checks"] = []): HttpResponse => ({
  method: "GET",
  url: "",
  status,
  statusText: "",
  headers: [],
  body: "",
  truncated: false,
  timeMs: 1,
  sizeBytes: 0,
  unresolved: [],
  warnings: [],
  captured: {},
  captureFailures: [],
  checks,
  critter: "bob",
  relogged: false,
});

const step = (index: number, patch: Partial<FlowStepResult>): FlowStepResult => ({
  index,
  request: `step${index}.yml`,
  title: `Step ${index}`,
  method: "GET",
  url: "",
  ok: true,
  skipped: false,
  response: response(200),
  error: null,
  watched: {},
  ...patch,
});

const run = (steps: FlowStepResult[]): FlowRun => ({
  at: 0,
  env: null,
  ok: steps.every((s) => s.ok),
  elapsedMs: 1240,
  steps,
});

describe("flow run summaries", () => {
  it("counts passes", () => {
    expect(flowRunSummary(run([step(0, {}), step(1, {})]))).toBe("2/2 passed · 1.24s");
    expect(flowRunSummary(run([]))).toBe("No steps · 1.24s");
  });

  it("names the first failure", () => {
    const failedCheck = { name: "res.status", ok: false, detail: "expected eq 403, got 200" };
    const summary = flowRunSummary(
      run([
        step(0, {}),
        step(1, { ok: false, response: response(200, [failedCheck]) }),
        step(2, { ok: false, skipped: true, response: null }),
      ]),
    );
    expect(summary).toBe("Failed at step 2 (step1.yml): res.status: expected eq 403, got 200");
  });

  it("explains a failed step by error, check or status", () => {
    expect(stepFailure(step(0, { ok: false, error: "no such file", response: null }))).toBe("no such file");
    expect(stepFailure(step(0, { ok: false, response: response(500) }))).toBe("status 500");
    expect(stepFailure(step(0, { ok: false, response: null }))).toBe("not run");
  });
});
