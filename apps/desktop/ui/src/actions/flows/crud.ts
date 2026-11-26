import { toast } from "sonner";
import { backend } from "@/api/client";
import { flowFilePath, uniqueFlowId } from "@/domain/critterId";
import { reportErrors, succeeded } from "@/lib/errors";
import { useCritterStore } from "@/stores/critterStore";
import { useUiStore } from "@/stores/uiStore";
import { refreshCritters, updateCritter } from "../critters";
import { writingFile } from "../saving";
import { openFlow, switchToFlow } from "./navigation";
import { currentProject, flowStore, isOpen } from "./shared";

/** How long Undo is offered after deleting a flow. */
const UNDO_MS = 6000;

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
      const id = uniqueFlowId(name, flowIds(critter));
      const created = await succeeded(
        () => backend.createFlow(project, critter, id, { name, steps: [] }),
        "Couldn't create the flow",
      );
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
    title: "Rename file",
    description: "Letters, digits, - and _ only. Its schedule and run log move with it.",
    initial: flow,
    onSubmit: async (to) => {
      if (!to) return false;
      if (to === flow) return true;
      const renamed = await succeeded(() => backend.renameFlow(project, critter, flow, to), "Rename failed");
      if (!renamed) return false;
      await reportErrors(refreshCritters);
      if (isOpen(critter, flow)) await openFlow(critter, to);
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
      const id = uniqueFlowId(copyName, flowIds(critter));
      const copied = await succeeded(
        () => backend.duplicateFlow(project, critter, flow, id, copyName),
        "Couldn't duplicate",
      );
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
  const deleted = await writingFile(flowFilePath(critter, flow), () =>
    succeeded(() => backend.deleteFlow(project, critter, flow), "Couldn't delete"),
  );
  if (!deleted) return;
  if (isOpen(critter, flow)) {
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
