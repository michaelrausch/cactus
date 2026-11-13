import type { LucideIcon } from "lucide-react";
import type { ComponentType } from "react";
import type { ToolStep } from "@/api/client";
import type { ToolDataflow } from "@/domain/flowDataflow";

/** A tool's id, as written after `tool:`. The generated bindings list them all. */
export type ToolId = ToolStep["tool"];

/** The step type of one tool, e.g. `ToolOf<"wait">` is `{ tool: "wait"; seconds?: number }`. */
export type ToolOf<T extends ToolId> = Extract<ToolStep, { tool: T }>;

export interface ToolEditorProps<T extends ToolId> {
  step: ToolOf<T>;
  /** Change this step, as an undoable flow edit. */
  update: (change: (step: ToolOf<T>) => ToolOf<T>) => void;
}

/** Everything the flow editor needs to know about one tool. */
export interface ToolDef<T extends ToolId> {
  label: string;
  icon: LucideIcon;
  /** The step added when it's dropped into a flow. */
  create: () => ToolOf<T>;
  /** A line about this step for its card, e.g. the URL it posts to. */
  summary?: (step: ToolOf<T>) => string;
  /** What it reads (`templates`), sets for later steps and gets wrong; each part optional. */
  dataflow?: (step: ToolOf<T>) => Partial<ToolDataflow>;
  /** Its settings, in the step pane. */
  Editor: ComponentType<ToolEditorProps<T>>;
}

/** Type a tool's definition; the registry in `index.ts` checks there's one per tool. */
export const defineTool = <T extends ToolId>(def: ToolDef<T>): ToolDef<T> => def;
