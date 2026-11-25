import { backend } from "@/api/client";
import { flowFilePath } from "@/domain/critterId";
import { flowFingerprint } from "@/domain/flowEdit";
import { reportErrors, succeeded } from "@/lib/errors";
import { refreshCritters } from "../critters";
import { refreshSchedules } from "../monitors";
import { markOwnWrite } from "../saving";
import { currentProject, fixFirst, flowStore, isOpen } from "./shared";

/** Save the open flow: the text as typed in YAML mode, or edited in place from the visual editor. */
export async function saveOpenFlow(): Promise<boolean> {
  const project = currentProject();
  const { open, model, mode, text, parseProblem } = flowStore();
  if (!project || !open || !model) return false;
  if (mode === "yaml" && parseProblem) {
    fixFirst(parseProblem.line);
    return false;
  }
  const { critter, flow } = open;
  const path = flowFilePath(critter, flow);
  const saved = await succeeded(async () => {
    markOwnWrite(path);
    if (mode === "yaml") await backend.saveFlowText(project, critter, flow, text);
    else await backend.saveFlow(project, critter, flow, model);
    markOwnWrite(path);
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
