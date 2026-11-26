import { backend, type FlowStepResult, type FlowStepStart, type FlowToolProgress } from "@/api/client";
import { reportErrors } from "@/lib/errors";
import { selectFlowDirty } from "@/stores/flowStore";
import { useResponseStore } from "@/stores/responseStore";
import { useUiStore } from "@/stores/uiStore";
import { track } from "../achievements";
import { selectScenario } from "../scenario";
import { switchToFlow } from "./navigation";
import { saveOpenFlow } from "./persist";
import { currentProject, flowStore, isOpen } from "./shared";

/** Show the flow and run it there, so each request can be watched as it goes. */
export async function runFlow(critter: string, flow: string): Promise<void> {
  if (await switchToFlow(critter, flow)) runOpenFlow();
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
    title: "Save changes?",
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
  if (!project || !open || !model) return;
  flowStore().runStarted(model.steps.length);
  const run = await reportErrors(() => backend.runFlow(project, open.critter, open.flow, null), "Run failed");
  // `null` when cancelled by another send.
  flowStore().runFinished(run ?? null);
  if (run) track("flowsRun");
}

/** A step of a run is sending (the `FlowStepStarted` event). */
export function flowStepStarted(critter: string, flow: string, start: FlowStepStart): void {
  if (isOpen(critter, flow) && flowStore().run.running) flowStore().stepStarted(start);
}

/** A step of a run finished (the `FlowProgress` event). */
export function flowStepFinished(critter: string, flow: string, step: FlowStepResult): void {
  if (isOpen(critter, flow) && flowStore().run.running) flowStore().stepFinished(step);
}

/** A long-running tool step reported how far it has got (the `FlowToolProgress` event). */
export function flowToolProgressed(progress: FlowToolProgress): void {
  if (isOpen(progress.critter, progress.flow) && flowStore().run.running) flowStore().toolProgressed(progress);
}

/** Show a step's response in the response pane, next to its request. */
export async function showStepResponse(result: FlowStepResult): Promise<void> {
  if (!result.response) return;
  await selectScenario(result.request);
  useResponseStore.getState().responseShown(result.response);
  useUiStore.getState().setResponseVisible(true);
}
