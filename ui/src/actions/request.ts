import { toast } from "sonner";
import { backend, backendEvents, type RunResult, type ScenarioPath } from "@/api/client";
import { isWithinDir } from "@/domain/paths";
import { errorMessage, reportErrors } from "@/lib/errors";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { useRunStore } from "@/stores/runStore";
import { flushPendingSave } from "./autosave";
import { selectScenario } from "./scenario";

/** Send the open scenario as currently edited, even if not saved yet. */
export async function sendCurrentScenario(): Promise<void> {
  const { project } = useProjectStore.getState();
  const { path, scenario } = useEditorStore.getState();
  const responses = useResponseStore.getState();
  if (!project || !path || !scenario || responses.sending) return;

  responses.sendStarted();
  try {
    const response = await backend.sendRequest(project, path, scenario);
    if (!response) return; // Cancelled.
    responses.sendSucceeded(path, response);
    responses.setHistory(await backend.loadHistory(project, path));
  } catch (error) {
    responses.sendFailed(errorMessage(error));
  } finally {
    responses.sendFinished();
  }
}

/** Cancels whichever send or folder run is in flight. */
export const cancelInFlight = (): Promise<unknown> => reportErrors(() => backend.cancelRequest());

function recordRunResult(result: RunResult): void {
  useRunStore.getState().resultArrived(result);
  if (result.response) useResponseStore.getState().statusSeen(result.relPath, result.response.status);
}

/** Run every scenario under `dir`, streaming results into the run dialog. */
export async function runFolder(dir: string): Promise<void> {
  const { project, entries } = useProjectStore.getState();
  if (!project) return;
  await flushPendingSave();
  const total = entries.filter((entry) => isWithinDir(entry.relPath, dir)).length;
  if (total === 0) {
    toast.info("Nothing to run");
    return;
  }

  useRunStore.getState().started(dir, total);
  const unlisten = await backendEvents.runProgress.listen((event) => recordRunResult(event.payload));
  try {
    await backend.runFolder(project, dir);
  } catch (error) {
    toast.error(errorMessage(error));
  } finally {
    unlisten();
    useRunStore.getState().finished();
    await reportErrors(() => reloadOpenHistory(project));
  }
}

/** A run may have added history for the open scenario. */
async function reloadOpenHistory(project: string): Promise<void> {
  const { path } = useEditorStore.getState();
  if (path) useResponseStore.getState().showHistory(await backend.loadHistory(project, path));
}

/** Results can only be opened once the run is over. */
export function openRunResult(path: ScenarioPath): void {
  const { run, dismissed } = useRunStore.getState();
  if (!run?.done) return;
  dismissed();
  void selectScenario(path);
}

export function dismissRun(): void {
  const { run, dismissed } = useRunStore.getState();
  if (run?.done) dismissed();
}
