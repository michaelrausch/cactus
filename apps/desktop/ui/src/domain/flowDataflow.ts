import type { Action, FlowStep, Scenario, Step, ToolStep, VarSource } from "@/api/client";
import { isToolStep } from "./flowEdit";
import { ownVariableNames } from "./scenario";
import { templateParts } from "./template";
import { varSourceLabel } from "./varSources";

/** Where a step's `{{name}}` gets its value, highest precedence first. */
export type UseSource =
  /** The step's own `with`. */
  | { kind: "with" }
  /** Captured by an earlier step of this flow. */
  | { kind: "step"; step: number }
  /** The request's own `runtime.variables`. */
  | { kind: "request" }
  /** The critter's env, variables, secrets or context. */
  | { kind: "scope"; label: string }
  | { kind: "missing" };

export interface StepUse {
  name: string;
  source: UseSource;
}

export interface FlowProblem {
  step: number;
  message: string;
}

export interface Dataflow {
  /** Per step: each `{{name}}` it uses and where the value comes from. */
  uses: StepUse[][];
  /** Per step: the names it captures (its `capture` and its request's set-variable actions). */
  captures: string[][];
  /** Between step i and i+1: names an earlier step captured that a later one uses. */
  passing: string[][];
  problems: FlowProblem[];
}

/**
 * A step's request: the scenario, `null` when the file doesn't exist or
 * doesn't parse, `undefined` while it's still loading.
 */
export type LoadedRequests = Readonly<Record<string, Scenario | null | undefined>>;

const isSetVariable = (action: Action): action is Extract<Action, { type: "set-variable" }> =>
  typeof action === "object" && action !== null && !Array.isArray(action) && action["type"] === "set-variable";

/** Names a request's own set-variable actions capture after the response. Mirrors `response_actions` in Rust. */
export function requestCaptures(scenario: Scenario): string[] {
  return (scenario.runtime?.actions ?? [])
    .filter(isSetVariable)
    .filter((set) => !set.disabled && set.phase !== "before-request")
    .map((set) => set.variable.name.trim())
    .filter(Boolean);
}

/** Every text in a request that `{{name}}` substitution reads: URL, headers, path params, body and auth. */
export function requestTemplates(scenario: Scenario): string[] {
  const { http } = scenario;
  const texts = [http.url];
  for (const header of http.headers ?? []) if (!header.disabled) texts.push(header.name, header.value);
  for (const param of http.params ?? []) if (!param.disabled && param.type === "path") texts.push(param.value);
  const body = http.body;
  if (body && typeof body === "object" && !Array.isArray(body) && "type" in body) {
    const data = (body as { data?: unknown }).data;
    if (typeof data === "string") texts.push(data);
    else if (Array.isArray(data)) {
      for (const field of data as { name?: string; value?: unknown; disabled?: boolean }[]) {
        if (field.disabled) continue;
        if (field.name) texts.push(field.name);
        if (typeof field.value === "string") texts.push(field.value);
      }
    }
  }
  const auth = http.auth;
  if (auth && typeof auth === "object" && !Array.isArray(auth) && "type" in auth) {
    for (const value of Object.values(auth)) if (typeof value === "string") texts.push(value);
  }
  return texts;
}

/** The distinct `{{names}}` in `texts`, in order of appearance. */
export function placeholderNames(texts: readonly string[]): string[] {
  const names = new Set<string>();
  for (const text of texts) for (const part of templateParts(text)) if (part.placeholder) names.add(part.placeholder);
  return [...names];
}

/** What a tool step reads, sets and gets wrong, as its editor in the tool registry says. */
export interface ToolDataflow {
  /** Texts that may hold `{{names}}`. */
  templates: string[];
  /** Names it sets for later steps. */
  sets: string[];
  problems: string[];
}

export type ToolDataflowOf = (step: ToolStep) => ToolDataflow;

const noToolDataflow: ToolDataflowOf = () => ({ templates: [], sets: [], problems: [] });

/** The parts of a request step that matter here, in the shape tools give theirs. */
function requestDataflow(step: FlowStep, index: number, requests: LoadedRequests, problems: FlowProblem[]) {
  const path = step.request.trim();
  const scenario = path ? requests[path] : null;
  if (!path) problems.push({ step: index, message: "No request" });
  else if (scenario === null) problems.push({ step: index, message: `${path} doesn't exist or doesn't parse` });
  return {
    scenario,
    templates: [...(scenario ? requestTemplates(scenario) : []), ...Object.values(step.with ?? {})],
    sets: [...Object.keys(step.capture ?? {}), ...(scenario ? requestCaptures(scenario) : [])],
    own: new Set(scenario ? ownVariableNames(scenario.runtime?.variables) : []),
    overrides: new Set(Object.keys(step.with ?? {})),
  };
}

/**
 * Follow values through a flow: what each step uses and captures, what
 * passes between steps, and what's wrong (a request that's gone, a name
 * nothing sets). `scope` is what the flow's critter sees before any step
 * runs; `critter` names it in labels. `toolDataflow` describes tool steps.
 */
export function flowDataflow(
  steps: readonly Step[],
  requests: LoadedRequests,
  scope: Readonly<Record<string, VarSource>>,
  critter?: string,
  toolDataflow: ToolDataflowOf = noToolDataflow,
): Dataflow {
  const uses: StepUse[][] = [];
  const captures: string[][] = [];
  const problems: FlowProblem[] = [];
  const passing: string[][] = steps.slice(1).map(() => []);
  /** The latest step to capture each name so far. */
  const capturedBy = new Map<string, number>();

  steps.forEach((step, index) => {
    let scenario: Scenario | null | undefined = null;
    let texts: string[];
    let sets: string[];
    let own = new Set<string>();
    let overrides = new Set<string>();
    if (isToolStep(step)) {
      const tool = toolDataflow(step);
      for (const message of tool.problems) problems.push({ step: index, message });
      ({ templates: texts, sets } = tool);
    } else {
      ({ scenario, templates: texts, sets, own, overrides } = requestDataflow(step, index, requests, problems));
    }
    const stepUses = placeholderNames(texts).map((name): StepUse => {
      if (overrides.has(name)) return { name, source: { kind: "with" } };
      const from = capturedBy.get(name);
      if (from !== undefined) {
        for (let gap = from; gap < index; gap++) passing[gap]?.push(name);
        return { name, source: { kind: "step", step: from } };
      }
      if (own.has(name)) return { name, source: { kind: "request" } };
      const lower = scope[name];
      if (lower) return { name, source: { kind: "scope", label: varSourceLabel(lower, critter) } };
      // A request still loading can't be judged yet.
      if (scenario !== undefined) problems.push({ step: index, message: `{{${name}}} isn't set by anything` });
      return { name, source: { kind: "missing" } };
    });
    uses.push(stepUses);

    const stepCaptures = [...new Set(sets.map((name) => name.trim()).filter(Boolean))];
    captures.push(stepCaptures);
    for (const name of stepCaptures) capturedBy.set(name, index);
  });

  const distinctPassing = passing.map((names) => Array.from(new Set(names)));
  return { uses, captures, passing: distinctPassing, problems };
}
