import { backend, type Flow, type Scenario } from "@/api/client";
import { flowFingerprint, stepRequest } from "@/domain/flowEdit";
import { reportErrors, warnQuietly } from "@/lib/errors";
import type { FlowMode } from "@/stores/flowStore";
import { currentProject, fixFirst, flowStore } from "./shared";

/** How long typing in the YAML editor pauses before it is parsed. */
const PARSE_DELAY_MS = 200;

/**
 * Load the requests the flow's steps use, for data flow and problems. A
 * request that can't be read is recorded as null. With `reload`, requests
 * already loaded are read again.
 */
export async function loadStepRequests(model: Flow, reload = false): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const known = flowStore().requests;
  const paths = [...new Set(model.steps.map(stepRequest).filter(Boolean))].filter((path) => reload || !(path in known));
  if (paths.length === 0) return;
  const loaded = await Promise.all(
    paths.map(async (path): Promise<[string, Scenario | null]> => [
      path,
      await backend.loadScenario(project, path).catch(() => null),
    ]),
  );
  flowStore().requestsLoaded(Object.fromEntries(loaded));
}

/** A visual edit to the open flow: undoable, and it loads any request newly added. */
export function editFlow(change: (flow: Flow) => Flow): void {
  const { model, mode } = flowStore();
  if (!model || mode !== "visual") return;
  const next = change(model);
  if (flowFingerprint(next) === flowFingerprint(model)) return;
  flowStore().edited(next);
  void loadStepRequests(next);
}

export const undoFlowEdit = (): void => flowStore().undone();
export const redoFlowEdit = (): void => flowStore().redone();

let parseTimer: ReturnType<typeof setTimeout> | undefined;

/** Typing in the YAML editor: parsed once typing pauses, so data flow and problems stay live. */
export function editFlowText(text: string): void {
  flowStore().textEdited(text);
  clearTimeout(parseTimer);
  parseTimer = setTimeout(() => void parseFlowText(text), PARSE_DELAY_MS);
}

async function parseFlowText(text: string): Promise<void> {
  const result = await warnQuietly(() => backend.parseFlow(text), "Couldn't check the flow");
  // Typing may have moved on while this was parsed.
  if (!result || flowStore().text !== text) return;
  if (result.kind === "ok") {
    flowStore().textParsed(result.flow, null);
    void loadStepRequests(result.flow);
  } else {
    flowStore().textParsed(null, { message: result.message, line: result.line, column: result.column });
  }
}

/**
 * Switch between the visual editor and YAML. YAML shows the file as written,
 * or the unsaved edits as YAML; back to visual is blocked while it doesn't parse.
 */
export async function setFlowMode(mode: FlowMode): Promise<void> {
  const project = currentProject();
  const state = flowStore();
  if (!project || !state.open || !state.model || mode === state.mode) return;
  if (mode === "visual") {
    if (state.parseProblem) return void fixFirst(state.parseProblem.line);
    flowStore().modeChanged("visual");
    return;
  }
  const { critter, flow } = state.open;
  const model = state.model;
  const dirty = flowFingerprint(model) !== state.savedFingerprint;
  const text = await reportErrors(
    () => (dirty ? backend.flowToYaml(model) : backend.loadFlowText(project, critter, flow)),
    "Couldn't show the YAML",
  );
  if (text !== undefined) flowStore().modeChanged("yaml", text);
}
