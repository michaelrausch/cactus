/**
 * Every flow tool the editor offers. To add one, write `<id>.tsx` with
 * `defineTool` next to these, and list it here (or in `edition/pro` for a
 * Pro tool); until then this file doesn't typecheck, since `CORE_TOOLS`
 * needs one entry per free tool the backend has (see
 * `core/src/tools/mod.rs`).
 */
import { PRO_TOOLS } from "@edition";
import type { ProToolId } from "@/edition/types";
import type { ToolStep } from "@/api/client";
import type { ToolDataflow } from "@/domain/flowDataflow";
import { assert } from "./assert";
import { display } from "./display";
import { lockedTool } from "./locked";
import { notify } from "./notify";
import { open } from "./open";
import { pick } from "./pick";
import { poll } from "./poll";
import { prompt } from "./prompt";
import { run } from "./run";
import { set } from "./set";
import type { ToolDef, ToolId, ToolOf } from "./types";
import { wait } from "./wait";

const CORE_TOOLS: { readonly [T in Exclude<ToolId, ProToolId>]: ToolDef<T> } = {
  prompt,
  pick,
  display,
  set,
  assert,
  poll,
  wait,
  run,
  open,
  notify,
};

/** The tools this edition has. */
export const TOOLS: { readonly [T in ToolId]?: ToolDef<T> } = { ...CORE_TOOLS, ...PRO_TOOLS };

export const TOOL_IDS = Object.keys(TOOLS) as ToolId[];

/** Every tool's palette section, in order. Every tool is in exactly one (checked in `tools.test.ts`). */
const ALL_SECTIONS: readonly { label: string; tools: readonly ToolId[] }[] = [
  { label: "Input", tools: ["prompt", "pick", "set"] },
  { label: "Flow", tools: ["assert", "poll", "wait", "run"] },
  { label: "Output", tools: ["display", "open", "webhook", "slack", "notify"] },
  { label: "Security", tools: ["fuzz", "headers", "cert", "ratelimit", "loadtest"] },
];

/** The palette's sections, with the tools this edition has. */
export const TOOL_SECTIONS = ALL_SECTIONS.map(({ label, tools }) => ({
  label,
  tools: tools.filter((id) => id in TOOLS),
})).filter((section) => section.tools.length > 0);

export const isToolId = (id: string): id is ToolId => id in TOOLS;

/** The definition of the tool `id`, or a locked stand-in for one this edition doesn't have. */
export const toolDef = <T extends ToolId>(id: T): ToolDef<T> =>
  (TOOLS[id] as ToolDef<T> | undefined) ?? (lockedTool(id) as unknown as ToolDef<T>);

/** The definition of the tool `step` is. */
export const toolOf = <T extends ToolId>(step: ToolOf<T>): ToolDef<T> => toolDef(step.tool as T);

/** What a tool step reads and sets, for `flowDataflow`. */
export function toolDataflow(step: ToolStep): ToolDataflow {
  const flow = toolOf(step).dataflow?.(step) ?? {};
  return { templates: flow.templates ?? [], sets: flow.sets ?? [], problems: flow.problems ?? [] };
}

export type { ToolDef, ToolEditorProps, ToolId, ToolOf } from "./types";
