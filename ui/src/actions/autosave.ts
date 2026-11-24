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

/**
 * How long after saving a file its change event counts as our own. The
 * watcher debounces by 150 ms, so its event arrives well within this.
 */
const OWN_WRITE_WINDOW_MS = 1000;

let pending: PendingSave | null = null;
/** When each scenario was last written by autosave. */
const lastWrittenAt = new Map<ScenarioPath, number>();
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
    lastWrittenAt.set(save.path, Date.now());
    await backend.saveScenario(save.project, save.path, save.scenario);
    lastWrittenAt.set(save.path, Date.now());
    useProjectStore.getState().entrySaved(save.path, save.scenario);
  }, "Save failed");
}

/** Whether autosave just wrote `path`, so a change event for it is our own echo. */
export const isOwnRecentWrite = (path: string): boolean =>
  Date.now() - (lastWrittenAt.get(path as ScenarioPath) ?? -Infinity) < OWN_WRITE_WINDOW_MS;

export const hasPendingSave = (): boolean => pending !== null;

/** Drop a pending save for a scenario that is about to be deleted. */
export function discardPendingSave(path: ScenarioPath): void {
  if (pending?.path !== path) return;
  pending = null;
  clearTimeout(timer);
}
