import { backend } from "@/api/client";
import { flowFilePath } from "@/domain/critterId";
import { flowFingerprint } from "@/domain/flowEdit";
import { reportErrors, succeeded } from "@/lib/errors";
import { selectFlowDirty } from "@/stores/flowStore";
import { refreshCritters } from "../critters";
import { refreshSchedules } from "../monitors";
import { writingFile } from "../saving";
import { currentProject, fixFirst, flowStore, isOpen } from "./shared";

/** How long the save button shows its tick. */
const SAVED_FLASH_MS = 1200;

let flashTimer: ReturnType<typeof setTimeout> | undefined;

/** Save the open flow: the text as typed in YAML mode, or edited in place from the visual editor. */
export async function saveOpenFlow(): Promise<boolean> {
  const project = currentProject();
  const { open } = flowStore();
  if (!project || !open) return false;
  const { critter, flow } = open;
  clearTimeout(flashTimer);
  flowStore().saveStatusChanged("saving");
  const saved = await writingFile(flowFilePath(critter, flow), () => writeFlow(project, critter, flow));
  if (!isOpen(critter, flow)) return saved;
  flowStore().saveStatusChanged(saved ? "saved" : "idle");
  if (saved) flashTimer = setTimeout(() => flowStore().saveStatusChanged("idle"), SAVED_FLASH_MS);
  return saved;
}

/** Runs after any earlier save of the flow, so it sees the baseline that save left. */
async function writeFlow(project: string, critter: string, flow: string): Promise<boolean> {
  if (!isOpen(critter, flow)) return false;
  const state = flowStore();
  const { model, mode, text, parseProblem, conflict } = state;
  if (!model) return false;
  if (mode === "yaml" && parseProblem) {
    fixFirst(parseProblem.line);
    return false;
  }
  if (!selectFlowDirty(state) && !conflict) return true;
  const saved = await succeeded(async () => {
    if (mode === "yaml") await backend.saveFlowText(project, critter, flow, text);
    else await backend.saveFlow(project, critter, flow, model);
    const [onDisk, diskText] = await Promise.all([
      backend.loadFlow(project, critter, flow),
      backend.loadFlowText(project, critter, flow),
    ]);
    // The user may have switched flows, or kept typing, while this saved.
    if (!isOpen(critter, flow)) return;
    const now = flowStore();
    const untouched = now.text === text && now.model && flowFingerprint(now.model) === flowFingerprint(model);
    if (untouched) now.saved(onDisk, diskText);
    else now.savedBehindEdits(onDisk, diskText);
  }, "Couldn't save the flow");
  if (!saved) return false;
  // The sidebar shows which flows have triggers; a saved schedule starts running.
  void reportErrors(refreshCritters);
  void refreshSchedules();
  return true;
}
