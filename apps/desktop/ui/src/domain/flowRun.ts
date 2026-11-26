import type { FlowRun, FlowStepResult } from "@/api/client";

/** Why a step failed: its error, its first failed check, or its status. Mirrors `FlowStepResult::reason` in Rust. */
export function stepFailure(step: FlowStepResult): string {
  if (step.error) return step.error;
  const response = step.response;
  if (!response) return "not run";
  const check = response.checks.find((c) => !c.ok);
  return check ? `${check.name}: ${check.detail}` : `status ${response.status}`;
}

/** The request a step sent, or a tool step's title. Mirrors `FlowStepResult::label` in Rust. */
export const stepLabel = (step: FlowStepResult): string => step.request || step.title;

const seconds = (ms: number): string => `${(ms / 1000).toFixed(2)}s`;

/** `"3/3 passed · 1.24s"`, or `"Failed at step 2 (admin.yml): res.status: expected eq 403, got 200"`. */
export function flowRunSummary(run: FlowRun): string {
  const failed = run.steps.find((step) => !step.ok && !step.skipped);
  if (failed) return `Failed at step ${failed.index + 1} (${stepLabel(failed)}): ${stepFailure(failed)}`;
  if (run.steps.length === 0) return `No steps · ${seconds(run.elapsedMs)}`;
  const passed = run.steps.filter((step) => step.ok).length;
  return `${passed}/${run.steps.length} passed · ${seconds(run.elapsedMs)}`;
}
