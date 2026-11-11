/**
 * Saving and reloading scenarios. Nothing is written until the user asks:
 * edits live in `editorStore` buffers, which are dirty until saved.
 */
import { toast } from "sonner";
import { backend, type Scenario, type ProjectPath } from "@/api/client";
import { reportErrors } from "@/lib/errors";
import { dirtyPaths, isDirty, sameScenario, useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";

/**
 * How long after saving a file its change event counts as our own. The
 * watcher debounces by 150 ms, so its event arrives well within this.
 */
const OWN_WRITE_WINDOW_MS = 1000;

const lastWrittenAt = new Map<string, number>();
/** Paths being written and read back; the buffer's baseline isn't settled yet. */
const savesInFlight = new Set<string>();

/**
 * Whether we just wrote `path`, so its change event is probably our own echo.
 * Only a hint, for skipping work: an outside edit in the same second matches too.
 */
export const isOwnRecentWrite = (path: string): boolean =>
  Date.now() - (lastWrittenAt.get(path) ?? -Infinity) < OWN_WRITE_WINDOW_MS;

/** Record that we're writing `path` (a flow, say), so its change event is taken as our own echo. */
export const markOwnWrite = (path: string): void => {
  lastWrittenAt.set(path, Date.now());
};

/** Whether a save of `path` is under way. */
export const isSaving = (path: string): boolean => savesInFlight.has(path);

const buffers = () => useEditorStore.getState().buffers;
const currentProject = (): string | null => useProjectStore.getState().project;

export const hasUnsavedChanges = (): boolean => dirtyPaths(buffers()).length > 0;

/**
 * Write the buffer for `path` (default: the open scenario). If the file
 * changed on disk since it was loaded, the first attempt stops and marks a
 * conflict; saving again overwrites. Resolves to whether it is now saved.
 */
export async function saveScenario(path: ProjectPath | null = useEditorStore.getState().path): Promise<boolean> {
  const project = currentProject();
  const buffer = path ? buffers()[path] : undefined;
  if (!project || !path || !buffer) return false;
  if (!isDirty(buffer) && !buffer.conflict) return true;

  const snapshot = buffer.scenario;
  const saved = await reportErrors(async () => {
    if (!buffer.conflict) {
      const onDisk = await backend.loadScenario(project, path).catch(() => null);
      if (onDisk && !sameScenario(onDisk, buffer.baseline)) {
        useEditorStore.getState().conflicted(path, "changed");
        toast.warning("File changed on disk", { description: "Reload it, or save again to overwrite." });
        return false;
      }
    }
    savesInFlight.add(path);
    try {
      lastWrittenAt.set(path, Date.now());
      await backend.saveScenario(project, path, snapshot);
      lastWrittenAt.set(path, Date.now());
      // The backend normalises on save (query params are synced with the URL), so
      // the file reads back slightly different from what was sent. Compare future
      // changes against what is really on disk, or every save looks like a clash.
      const written = await backend.loadScenario(project, path).catch(() => snapshot);
      useEditorStore.getState().saved(path, snapshot, written);
    } finally {
      savesInFlight.delete(path);
    }
    useProjectStore.getState().entrySaved(path, snapshot);
    if (buffer.conflict === "deleted") {
      useProjectStore.getState().entriesListed(await backend.listScenarios(project));
    }
    return true;
  }, "Save failed");
  return saved ?? false;
}

/** Save every dirty buffer. Resolves to whether all of them went through. */
export async function saveAll(paths: ProjectPath[] = dirtyPaths(buffers())): Promise<boolean> {
  const results = await Promise.all(paths.map((path) => saveScenario(path)));
  return results.every(Boolean);
}

/** Throw away the buffer's edits and show what is on disk. */
export async function reloadFromDisk(path: ProjectPath): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await reportErrors(async () => {
    useEditorStore.getState().reloaded(path, await backend.loadScenario(project, path));
  }, "Reload failed");
}

/**
 * Follow an external edit to open or modified scenarios. A clean buffer
 * reloads silently; one with unsaved edits is flagged as a conflict instead.
 */
export async function reloadIfChangedOnDisk(changedPaths: string[]): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await Promise.all(
    (changedPaths as ProjectPath[]).filter((path) => buffers()[path]).map((path) => followDisk(project, path)),
  );
}

async function followDisk(project: string, path: ProjectPath): Promise<void> {
  let onDisk: Scenario;
  try {
    onDisk = await backend.loadScenario(project, path);
  } catch {
    // Deleted or mid-write. Unsaved work is worth flagging; otherwise the list refresh covers deletion.
    if (isDirty(buffers()[path])) useEditorStore.getState().conflicted(path, "deleted");
    return;
  }
  const buffer = buffers()[path]; // May have changed while loading.
  if (!buffer || sameScenario(onDisk, buffer.baseline)) return;
  const editor = useEditorStore.getState();
  if (!isDirty(buffer)) {
    editor.reloaded(path, onDisk);
    if (editor.path === path) toast.info("Reloaded: file changed on disk");
  } else if (sameScenario(onDisk, buffer.scenario)) {
    editor.saved(path, onDisk); // The same edit made elsewhere.
  } else {
    editor.conflicted(path, "changed");
  }
}
