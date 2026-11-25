/**
 * Every flow tool the editor offers. To add one, write `<id>.tsx` with
 * `defineTool` next to these, and list it here; until then this file
 * doesn't typecheck, since `TOOLS` needs one entry per tool the backend
 * has (see `core/src/tools/mod.rs`).
 */
import type { ToolStep } from "@/api/client";
import type { ToolDataflow } from "@/domain/flowDataflow";
import { assert } from "./assert";
import { display } from "./display";
import { notify } from "./notify";
import { open } from "./open";
import { pick } from "./pick";
import { poll } from "./poll";
import { prompt } from "./prompt";
import { run } from "./run";
import { set } from "./set";
import { slack } from "./slack";
import type { ToolDef, ToolId, ToolOf } from "./types";
import { wait } from "./wait";
import { webhook } from "./webhook";

/** In the order the palette lists them. */
export const TOOLS: { readonly [T in ToolId]: ToolDef<T> } = {
  prompt,
  pick,
  display,
  set,
  assert,
  poll,
  wait,
  run,
  open,
  webhook,
  slack,
  notify,
};

export const TOOL_IDS = Object.keys(TOOLS) as ToolId[];

export const isToolId = (id: string): id is ToolId => id in TOOLS;

/** The definition of the tool `step` is. */
export const toolOf = <T extends ToolId>(step: ToolOf<T>): ToolDef<T> => TOOLS[step.tool as T];

/** What a tool step reads and sets, for `flowDataflow`. */
export function toolDataflow(step: ToolStep): ToolDataflow {
  const flow = toolOf(step).dataflow?.(step) ?? {};
  return { templates: flow.templates ?? [], sets: flow.sets ?? [], problems: flow.problems ?? [] };
}

export type { ToolDef, ToolEditorProps, ToolId, ToolOf } from "./types";
