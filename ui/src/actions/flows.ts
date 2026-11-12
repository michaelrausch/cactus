import { toast } from "sonner";
import {
  backend,
  type Flow,
  type FlowStepResult,
  type FlowStepStart,
  type ProjectPath,
  type Scenario,
} from "@/api/client";
import { flowFilePath, uniqueCritterId } from "@/domain/critterId";
import { flowFingerprint } from "@/domain/flowEdit";
import { reportErrors, warnQuietly } from "@/lib/errors";
import { useCritterStore } from "@/stores/critterStore";
import { selectFlowDirty, useFlowStore, type FlowMode } from "@/stores/flowStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { useUiStore } from "@/stores/uiStore";
import { refreshCritters, updateCritter } from "./critters";
import { refreshSchedules } from "./monitors";
import { markOwnWrite } from "./saving";
import { selectScenario } from "./scenario";
import { ensureUnlocked } from "./secrets";

/** How long typing in the YAML editor pauses before it is parsed. */
const PARSE_DELAY_MS = 200;
/** How long Undo is offered after deleting a flow. */
const UNDO_MS = 6000;

const currentProject = (): string | null => useProjectStore.getState().project;
const flowStore = () => useFlowStore.getState();

/** Open a critter's flow in the editor. */
export async function openFlow(critter: string, flow: string): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await reportErrors(async () => {
    const [model, text, scope] = await Promise.all([
      backend.loadFlow(project, critter, flow),
      backend.loadFlowText(project, critter, flow),
      backend.flowVariableSources(project, critter),
    ]);
    flowStore().opened({ critter, flow }, model, text, scope);
    await loadStepRequests(model, true);
  }, `Couldn't open ${flow}`);
}

const isOpen = (critter: string, flow: string): boolean => {
  const { open } = flowStore();
  return open?.critter === critter && open.flow === flow;
};

/** Show the open flow, `critter`'s, in the main pane. */
function showOpenFlow(critter: string): void {
  useCritterStore.getState().select(critter);
  useUiStore.getState().setView("flow");
}

/**
 * Show `flow` in the main pane, first asking about unsaved edits to a
 * different open flow. Resolves to whether it is showing; never resolves
 * if that question is cancelled.
 */
export function switchToFlow(critter: string, flow: string): Promise<boolean> {
  if (isOpen(critter, flow)) {
    showOpenFlow(critter);
    return Promise.resolve(true);
  }
  return new Promise((resolve) =>
    guardUnsaved(() => {
      void openFlow(critter, flow).then(() => {
        const opened = isOpen(critter, flow);
        if (opened) showOpenFlow(critter);
        resolve(opened);
      });
    }),
  );
}

/** Show the flow and run it there, so each request can be watched as it goes. */
export async function runFlow(critter: string, flow: string): Promise<void> {
  if (await switchToFlow(critter, flow)) runOpenFlow();
}

/** Run `leave` once unsaved edits are saved or discarded, or right away if there are none. */
function guardUnsaved(leave: () => void): void {
  if (!selectFlowDirty(flowStore())) {
    leave();
    return;
  }
  useUiStore.getState().askConfirm({
    title: "Save changes?",
    description: `You have unsaved changes to ${flowStore().model?.name || "this flow"}.`,
    action: "Save",
    onConfirm: async () => {
      if (await saveOpenFlow()) leave();
    },
    alternative: { action: "Discard", onConfirm: leave, destructive: true },
  });
}

/**
 * Load the requests the flow's steps use, for data flow and problems. A
 * request that can't be read is recorded as null. With `reload`, requests
 * already loaded are read again.
 */
async function loadStepRequests(model: Flow, reload = false): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const known = flowStore().requests;
  const paths = [...new Set(model.steps.map((step) => step.request.trim()).filter(Boolean))].filter(
    (path) => reload || !(path in known),
  );
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

const fixFirst = (line: number | null) =>
  toast.warning(line === null ? "Fix the YAML first" : `Fix line ${line} first`);

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

/** Save the open flow: the text as typed in YAML mode, or edited in place from the visual editor. */
export async function saveOpenFlow(): Promise<boolean> {
  const project = currentProject();
  const state = flowStore();
  if (!project || !state.open || !state.model) return false;
  if (state.mode === "yaml" && state.parseProblem) {
    fixFirst(state.parseProblem.line);
    return false;
  }
  const { critter, flow } = state.open;
  const path = flowFilePath(critter, flow);
  const saved = await reportErrors(async () => {
    markOwnWrite(path);
    if (state.mode === "yaml") await backend.saveFlowText(project, critter, flow, state.text);
    else await backend.saveFlow(project, critter, flow, state.model as Flow);
    markOwnWrite(path);
    const [model, text] = await Promise.all([
      backend.loadFlow(project, critter, flow),
      backend.loadFlowText(project, critter, flow),
    ]);
    flowStore().saved(model, text);
    return true;
  }, "Couldn't save the flow");
  if (!saved) return false;
  // The sidebar shows which flows have triggers; a saved schedule starts running.
  void reportErrors(refreshCritters);
  void refreshSchedules();
  return true;
}

/** Run the open flow (Ctrl+Enter). Flows run from disk, so unsaved edits are saved first or left out. */
export function runOpenFlow(): void {
  const state = flowStore();
  if (!state.open || state.run.running) return;
  if (!selectFlowDirty(state)) {
    void runSavedFlow();
    return;
  }
  useUiStore.getState().askConfirm({
    title: "Unsaved changes",
    description: "Flows run from the file on disk.",
    action: "Save and run",
    onConfirm: async () => {
      if (await saveOpenFlow()) await runSavedFlow();
    },
    alternative: { action: "Run saved version", onConfirm: runSavedFlow },
  });
}

async function runSavedFlow(): Promise<void> {
  const project = currentProject();
  const { open, model } = flowStore();
  if (!project || !open || !model || !(await ensureUnlocked())) return;
  flowStore().runStarted(model.steps.length);
  const run = await reportErrors(() => backend.runFlow(project, open.critter, open.flow, null), "Run failed");
  // `null` when cancelled by another send.
  flowStore().runFinished(run ?? null);
}

/** A step of a run is sending (the `FlowStepStarted` event). */
export function flowStepStarted(critter: string, flow: string, start: FlowStepStart): void {
  if (isOpen(critter, flow) && flowStore().run.running) flowStore().stepStarted(start);
}

/** A step of a run finished (the `FlowProgress` event). */
export function flowStepFinished(critter: string, flow: string, step: FlowStepResult): void {
  if (isOpen(critter, flow) && flowStore().run.running) flowStore().stepFinished(step);
}

/** Show a step's response in the response pane, next to its request. */
export async function showStepResponse(result: FlowStepResult): Promise<void> {
  if (!result.response) return;
  await selectScenario(result.request as ProjectPath);
  useResponseStore.getState().responseShown(result.response);
  useUiStore.getState().setResponseVisible(true);
}

/** Files changed on disk: reload the open flow (or flag a conflict) and any step request that changed. */
export async function flowFilesChanged(external: string[]): Promise<void> {
  const state = flowStore();
  if (!state.open || !state.model) return;
  const ownFile = flowFilePath(state.open.critter, state.open.flow);
  if (external.includes(ownFile)) {
    if (selectFlowDirty(state)) flowStore().conflicted();
    else await openFlow(state.open.critter, state.open.flow);
    return;
  }
  const changedRequests = external.filter((path) => path in state.requests);
  if (changedRequests.length > 0) await loadStepRequests(state.model, true);
}

/** Keep the unsaved edits and overwrite the file that changed on disk. */
export const overwriteFlow = (): Promise<boolean> => saveOpenFlow();

/** Drop the unsaved edits and load the file as it is on disk. */
export async function reloadFlow(): Promise<void> {
  const { open } = flowStore();
  if (open) await openFlow(open.critter, open.flow);
}

const flowIds = (critter: string): string[] =>
  useCritterStore
    .getState()
    .critters.find((c) => c.id === critter)
    ?.flows.map((f) => f.id) ?? [];

/** Ask for a name and create an empty flow for `critter`. */
export function createFlow(critter: string): void {
  const project = currentProject();
  if (!project) return;
  useUiStore.getState().askPrompt({
    title: "New flow",
    description: "Requests the critter sends in order, e.g. Login, or Staging health.",
    initial: "",
    placeholder: "Staging health",
    onSubmit: async (name) => {
      if (!name) return false;
      const id = uniqueCritterId(name, flowIds(critter));
      const created = await reportErrors(async () => {
        await backend.createFlow(project, critter, id, { name, steps: [] });
        return true;
      }, "Couldn't create the flow");
      if (!created) return false;
      await reportErrors(refreshCritters);
      void switchToFlow(critter, id);
      return true;
    },
  });
}

/** Ask for a new file name and rename a flow; its schedule, run log and login role follow. */
export function renameFlow(critter: string, flow: string): void {
  const project = currentProject();
  if (!project) return;
  useUiStore.getState().askPrompt({
    title: "Rename flow file",
    description: "Letters, digits, - and _ only. Its schedule and run log move with it.",
    initial: flow,
    onSubmit: async (to) => {
      if (!to) return false;
      if (to === flow) return true;
      const renamed = await reportErrors(async () => {
        await backend.renameFlow(project, critter, flow, to);
        return true;
      }, "Rename failed");
      if (!renamed) return false;
      await reportErrors(refreshCritters);
      const { open } = flowStore();
      if (open?.critter === critter && open.flow === flow) await openFlow(critter, to);
      return true;
    },
  });
}

/** Copy a flow under a new name, comments and all. */
export function duplicateFlow(critter: string, flow: string, name: string): void {
  const project = currentProject();
  if (!project) return;
  useUiStore.getState().askPrompt({
    title: `Duplicate ${name}`,
    initial: `${name} copy`,
    onSubmit: async (copyName) => {
      if (!copyName) return false;
      const id = uniqueCritterId(copyName, flowIds(critter));
      const copied = await reportErrors(async () => {
        const text = await backend.loadFlowText(project, critter, flow);
        await backend.saveFlowText(project, critter, id, text);
        const copy = await backend.loadFlow(project, critter, id);
        await backend.saveFlow(project, critter, id, { ...copy, name: copyName });
        return true;
      }, "Couldn't duplicate");
      if (!copied) return false;
      await reportErrors(refreshCritters);
      return true;
    },
  });
}

/** Make `flow` the critter's login flow, or none. */
export async function setLoginFlow(critter: string, flow: string | null): Promise<void> {
  await updateCritter(critter, ({ login: _previous, ...rest }) => (flow ? { ...rest, login: flow } : rest));
}

/** Delete a flow, keeping its text so Undo can write it back. */
export async function deleteFlow(critter: string, flow: string): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const text = await reportErrors(() => backend.loadFlowText(project, critter, flow), "Couldn't delete");
  if (text === undefined) return;
  const deleted = await reportErrors(async () => {
    markOwnWrite(flowFilePath(critter, flow));
    await backend.deleteFlow(project, critter, flow);
    return true;
  }, "Couldn't delete");
  if (!deleted) return;
  const { open } = flowStore();
  if (open?.critter === critter && open.flow === flow) {
    flowStore().closed();
    const ui = useUiStore.getState();
    if (ui.view === "flow") ui.setView("critter");
  }
  await reportErrors(refreshCritters);
  toast(`Deleted ${flow}`, {
    duration: UNDO_MS,
    action: {
      label: "Undo",
      onClick: () =>
        void reportErrors(async () => {
          await backend.saveFlowText(project, critter, flow, text);
          await refreshCritters();
        }, "Couldn't undo"),
    },
  });
}
