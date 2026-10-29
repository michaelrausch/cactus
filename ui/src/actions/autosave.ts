/**
 * Debounced scenario saving. Edits are written shortly after typing stops,
 * and any pending save is flushed before switching scenario or project.
 */
import { backend, type Scenario, type ScenarioPath } from "@/api/client";
import { reportErrors } from "@/lib/errors";
import { useProjectStore } from "@/stores/projectStore";

const AUTOSAVE_DELAY_MS = 300;

interface PendingSave {
  project: string;
  path: ScenarioPath;
  scenario: Scenario;
}

let pending: PendingSave | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;

export function scheduleSave(save: PendingSave): void {
  pending = save;
  clearTimeout(timer);
  timer = setTimeout(() => void flushPendingSave(), AUTOSAVE_DELAY_MS);
}

export async function flushPendingSave(): Promise<void> {
  clearTimeout(timer);
  const save = pending;
  pending = null;
  if (!save) return;
  await reportErrors(async () => {
    await backend.saveScenario(save.project, save.path, save.scenario);
    useProjectStore.getState().entrySaved(save.path, save.scenario);
  }, "Save failed");
}

export const hasPendingSave = (): boolean => pending !== null;

/** Drop a pending save for a scenario that is about to be deleted. */
export function discardPendingSave(path: ScenarioPath): void {
  if (pending?.path !== path) return;
  pending = null;
  clearTimeout(timer);
}
