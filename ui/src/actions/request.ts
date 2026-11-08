import { toast } from "sonner";
import { backend, backendEvents, type HttpResponse, type RunResult, type ScenarioPath } from "@/api/client";
import { isWithinDir } from "@/domain/paths";
import { errorMessage, reportErrors } from "@/lib/errors";
import { dirtyPaths, useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { useRunStore } from "@/stores/runStore";
import { useUiStore } from "@/stores/uiStore";
import { selectScenario } from "./scenario";
import { saveAll } from "./saving";
import { refreshVariableNames } from "./variables";

/** As kept by the backend (`core/src/history.rs`). */
const MAX_HISTORY_ENTRIES = 10;

/** A capture that found nothing leaves the variable stale, which is easy to miss. */
function alertCaptureFailures(response: HttpResponse): void {
  const failures = response.captureFailures;
  if (failures.length === 0) return;
  const names = failures.map((failure) => failure.name).join(", ");
  toast.warning(`Couldn't capture ${names}`, {
    description: failures.map((failure) => `${failure.name}: ${failure.reason}`).join("\n"),
    classNames: { description: "whitespace-pre-line font-mono" },
  });
}

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
    alertCaptureFailures(response);
    if (Object.keys(response.captured).length > 0) void refreshVariableNames();
    // The backend has just recorded it; prepending saves re-reading the whole file.
    if (useEditorStore.getState().path === path) {
      const { history, setHistory } = useResponseStore.getState();
      setHistory([{ at: Date.now(), response }, ...history].slice(0, MAX_HISTORY_ENTRIES));
    }
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
  const { project } = useProjectStore.getState();
  if (!project) return;
  const unsaved = dirtyPaths(useEditorStore.getState().buffers).filter((path) => isWithinDir(path, dir));
  if (unsaved.length === 0) return runSaved(dir);
  // Runs read from disk, so unsaved edits would silently not take part.
  useUiStore.getState().askConfirm({
    title: "Unsaved changes",
    description: `${unsaved.length === 1 ? unsaved[0] : `${unsaved.length} requests`} ha${unsaved.length === 1 ? "s" : "ve"} unsaved edits. A run uses the saved files.`,
    action: "Save and run",
    destructive: false,
    onConfirm: async () => {
      if (await saveAll(unsaved)) await runSaved(dir);
    },
    alternative: { action: "Run saved versions", onConfirm: () => runSaved(dir) },
  });
}

async function runSaved(dir: string): Promise<void> {
  const { project, entries } = useProjectStore.getState();
  if (!project) return;
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
    void refreshVariableNames();
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
