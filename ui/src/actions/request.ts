import { toast } from "sonner";
import { backend, backendEvents, type HttpResponse, type RunResult, type ProjectPath } from "@/api/client";
import { isWithinDir } from "@/domain/paths";
import { errorMessage, reportErrors, warnQuietly } from "@/lib/errors";
import { dirtyPaths, useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { useRunStore } from "@/stores/runStore";
import { useUiStore } from "@/stores/uiStore";
import { selectScenario } from "./scenario";
import { track } from "./achievements";
import { saveAll } from "./saving";
import { refreshVariableNames } from "./variables";

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

function trackResponse(status: number): void {
  track("requestsSent");
  if (status >= 500) track("serverErrors");
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
    trackResponse(response.status);
    alertCaptureFailures(response);
    if (Object.keys(response.captured).length > 0) void refreshVariableNames();
    // The backend has just recorded it, and trimmed what it keeps; show its list.
    const history = await warnQuietly(() => backend.loadHistory(project, path), "Couldn't reload history");
    if (history && useEditorStore.getState().path === path) useResponseStore.getState().historyUpdated(history);
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
  if (!result.response) return;
  useResponseStore.getState().statusSeen(result.relPath, result.response.status);
  trackResponse(result.response.status);
}

/** Run every scenario under `dir`, streaming results into the run dialog. */
export async function runFolder(dir: string): Promise<void> {
  const { project } = useProjectStore.getState();
  if (!project) return;
  const unsaved = dirtyPaths(useEditorStore.getState().buffers).filter((path) => isWithinDir(path, dir));
  if (unsaved.length === 0) return runSaved(dir);
  // Runs read from disk, so unsaved edits would silently not take part.
  useUiStore.getState().askConfirm({
    title: "Save changes?",
    description: `${unsaved.length === 1 ? unsaved[0] : `${unsaved.length} requests`} ha${unsaved.length === 1 ? "s" : "ve"} unsaved edits. A run uses the saved files.`,
    action: "Save and run",
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
  track("foldersRun");
  const unlisten = await backendEvents.runProgress.listen((event) => recordRunResult(event.payload));
  try {
    await reportErrors(() => backend.runFolder(project, dir), "Run failed");
  } finally {
    unlisten();
    useRunStore.getState().finished();
    void refreshVariableNames();
    await reportErrors(() => reloadOpenHistory(project));
  }
}

/** An agent sent `path` over MCP (the `HistoryChanged` event): show its response if it's open. */
export async function historyChanged(path: ProjectPath): Promise<void> {
  const { project } = useProjectStore.getState();
  if (!project) return;
  const history = await warnQuietly(() => backend.loadHistory(project, path), "Couldn't reload history");
  const latest = history?.[0];
  if (!history || !latest) return;
  const responses = useResponseStore.getState();
  responses.statusSeen(path, latest.response.status);
  if (useEditorStore.getState().path === path && !responses.sending) responses.historyLoaded(history);
}

/** A run may have added history for the open scenario. */
async function reloadOpenHistory(project: string): Promise<void> {
  const { path } = useEditorStore.getState();
  if (path) useResponseStore.getState().historyLoaded(await backend.loadHistory(project, path));
}

/** Results can only be opened once the run is over. */
export function openRunResult(path: ProjectPath): void {
  const { run, dismissed } = useRunStore.getState();
  if (!run?.done) return;
  dismissed();
  void selectScenario(path);
}

/** Close the folder run's results; a run still going stays open. */
export function dismissRun(): void {
  const { run, dismissed } = useRunStore.getState();
  if (run?.done) dismissed();
}
