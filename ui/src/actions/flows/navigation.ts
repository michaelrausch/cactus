import { backend } from "@/api/client";
import { flowFilePath } from "@/domain/critterId";
import { reportErrors } from "@/lib/errors";
import { useCritterStore } from "@/stores/critterStore";
import { selectFlowDirty } from "@/stores/flowStore";
import { useUiStore } from "@/stores/uiStore";
import { isSaving, saveGeneration } from "../saving";
import { loadStepRequests } from "./editing";
import { saveOpenFlow } from "./persist";
import { currentProject, flowStore, isOpen } from "./shared";

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

/** Show the open flow, `critter`'s, in the main pane. */
function showOpenFlow(critter: string): void {
  useCritterStore.getState().select(critter);
  useUiStore.getState().setView("flow");
}

/**
 * Run `leave` once unsaved edits are saved or discarded, or right away if
 * there are none. `stay` runs instead if the question is cancelled or the
 * save fails.
 */
function guardUnsaved(leave: () => void, stay: () => void): void {
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
      else stay();
    },
    alternative: { action: "Discard", onConfirm: leave, destructive: true },
    onCancel: stay,
  });
}

/**
 * Show `flow` in the main pane, first asking about unsaved edits to a
 * different open flow. Resolves to whether it is showing: `false` if that
 * question is cancelled, or the save or the open fails.
 */
export function switchToFlow(critter: string, flow: string): Promise<boolean> {
  if (isOpen(critter, flow)) {
    showOpenFlow(critter);
    return Promise.resolve(true);
  }
  return new Promise((resolve) =>
    guardUnsaved(
      () => {
        void openFlow(critter, flow).then(() => {
          const opened = isOpen(critter, flow);
          if (opened) showOpenFlow(critter);
          resolve(opened);
        });
      },
      () => resolve(false),
    ),
  );
}

/** Files changed on disk: reload the open flow (or flag a conflict) and any step request that changed. */
export async function flowFilesChanged(changed: string[]): Promise<void> {
  const project = currentProject();
  const state = flowStore();
  if (!project || !state.open || !state.model) return;
  const { critter, flow } = state.open;
  if (changed.includes(flowFilePath(critter, flow)) && (await followFlowFile(project, critter, flow))) return;
  const now = flowStore();
  if (!isOpen(critter, flow) || !now.model) return;
  if (changed.some((path) => path in now.requests)) await loadStepRequests(now.model, true);
}

/**
 * The open flow's file changed. Compared with what was last loaded or saved,
 * so our own save's echo changes nothing however late it arrives. Resolves to
 * whether the flow was reloaded.
 */
async function followFlowFile(project: string, critter: string, flow: string): Promise<boolean> {
  const path = flowFilePath(critter, flow);
  // A save's own read-back settles the baseline, and is newer than anything read here.
  if (isSaving(path)) return false;
  const generation = saveGeneration(path);
  const onDisk = await backend.loadFlowText(project, critter, flow).catch(() => null);
  if (!isOpen(critter, flow) || isSaving(path) || saveGeneration(path) !== generation) return false;
  const state = flowStore();
  if (onDisk === state.savedText) return false;
  if (selectFlowDirty(state)) {
    state.conflicted();
    return false;
  }
  await openFlow(critter, flow);
  return true;
}

/** Keep the unsaved edits and overwrite the file that changed on disk. */
export const overwriteFlow = (): Promise<boolean> => saveOpenFlow();

/** Drop the unsaved edits and load the file as it is on disk. */
export async function reloadFlow(): Promise<void> {
  const { open } = flowStore();
  if (open) await openFlow(open.critter, open.flow);
}
