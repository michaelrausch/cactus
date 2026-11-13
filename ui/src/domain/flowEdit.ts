import type { Flow, FlowStep, Step, ToolStep } from "@/api/client";
import { assertionsFromRows, rowsFromAssertions } from "./assertions";
import { filledRows, recordFromRows, rowsFromRecord, type Row } from "./rows";

/**
 * Edits to a flow, as pure functions. Like requests, a flow is held exactly
 * as the backend writes it: empty maps and lists are left out, so a save
 * that changes nothing reads back the same.
 */

const clamp = (index: number, length: number) => Math.max(0, Math.min(index, length));

/** For dirty checks. Plain JSON, so key order counts (unlike the editor store's `stableStringify`). */
export const flowFingerprint = (flow: Flow): string => JSON.stringify(flow);

export const isToolStep = (step: Step): step is ToolStep => "tool" in step;

/** The request a step sends, trimmed; empty for a tool. */
export const stepRequest = (step: Step): string => (isToolStep(step) ? "" : step.request.trim());

function insertAt(flow: Flow, index: number, step: Step): Flow {
  const steps = [...flow.steps];
  steps.splice(clamp(index, steps.length), 0, step);
  return { ...flow, steps };
}

export const insertStep = (flow: Flow, index: number, request: string): Flow => insertAt(flow, index, { request });

export const insertTool = (flow: Flow, index: number, tool: ToolStep): Flow => insertAt(flow, index, tool);

/** Move the step at `from` so it ends up at index `to`. */
export function moveStep(flow: Flow, from: number, to: number): Flow {
  const steps = [...flow.steps];
  const [step] = steps.splice(from, 1);
  if (!step) return flow;
  steps.splice(clamp(to, steps.length), 0, step);
  return { ...flow, steps };
}

export function removeStep(flow: Flow, index: number): Flow {
  return { ...flow, steps: flow.steps.filter((_, i) => i !== index) };
}

/** Change the request step at `index`; a tool there is left alone. */
export function updateStep(flow: Flow, index: number, change: (step: FlowStep) => FlowStep): Flow {
  return { ...flow, steps: flow.steps.map((step, i) => (i === index && !isToolStep(step) ? change(step) : step)) };
}

/** Change the tool step at `index`; a request there is left alone. */
export function updateTool<T extends ToolStep>(flow: Flow, index: number, change: (tool: T) => T): Flow {
  // The caller knows which tool it edits; `tool` stays as it was.
  return { ...flow, steps: flow.steps.map((step, i) => (i === index && isToolStep(step) ? change(step as T) : step)) };
}

/** Empty as the backend sees it, which leaves the field out when writing. */
const isEmpty = (value: unknown): boolean =>
  value === undefined ||
  value === null ||
  value === "" ||
  value === false ||
  value === 0 ||
  (Array.isArray(value) ? value.length === 0 : typeof value === "object" && Object.keys(value).length === 0);

/**
 * `item` with `key` set to `value`, or left out when that's empty, as every
 * tool field is written. A field already there keeps its place, so the
 * flow's fingerprint doesn't change on a round trip.
 */
export function setField<T extends object, K extends keyof T>(item: T, key: K, value: T[K] | undefined): T {
  if (!isEmpty(value)) return { ...item, [key]: value };
  const { [key]: _previous, ...rest } = item;
  // TypeScript can't see that dropping an optional key leaves a T.
  return rest as T;
}

type StepMap = "with" | "capture";

/** `step` with `field` set, or removed when empty. */
function withField<K extends keyof FlowStep>(step: FlowStep, field: K, value: FlowStep[K] | undefined): FlowStep {
  const { [field]: _previous, ...rest } = step;
  const empty =
    value === undefined ||
    (Array.isArray(value) ? value.length === 0 : typeof value === "object" && Object.keys(value).length === 0);
  // TypeScript can't see that removing then re-adding `field` keeps `request`.
  return (empty ? rest : { ...rest, [field]: value }) as FlowStep;
}

/** `expect: 403`, or none. */
export const withExpect = (step: FlowStep, status: number | null): FlowStep =>
  withField(step, "expect", status === null ? undefined : status);

export const stepMapRows = (step: FlowStep, field: StepMap): Row[] => rowsFromRecord(step[field]);

/** `with` or `capture` from table rows; rows without a name are left out. */
export const withStepMapRows = (step: FlowStep, field: StepMap, rows: Row[]): FlowStep =>
  withField(step, field, recordFromRows(filledRows(rows)));

export const stepAssertRows = (step: FlowStep): Row[] => rowsFromAssertions(step.assert);

export const withStepAssertRows = (step: FlowStep, rows: Row[]): FlowStep =>
  withField(step, "assert", assertionsFromRows(filledRows(rows)));

export const withWatch = (step: FlowStep, expressions: string[]): FlowStep =>
  withField(step, "watch", expressions.map((e) => e.trim()).filter(Boolean));

/** Add `name: expression` to the step's captures. */
export const withCapture = (step: FlowStep, name: string, expression: string): FlowStep =>
  withField(step, "capture", { ...step.capture, [name]: expression });

/** Add `{{name}}`'s value for this step only. */
export const withOverride = (step: FlowStep, name: string, value = ""): FlowStep =>
  withField(step, "with", { ...step.with, [name]: value });
