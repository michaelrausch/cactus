import { toast } from "sonner";
import { backend, type Scenario, type ScenarioPath } from "@/api/client";
import { withJsonExtension } from "@/domain/paths";
import { reportErrors } from "@/lib/errors";
import { stableStringify } from "@/lib/format";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { useUiStore } from "@/stores/uiStore";
import { discardPendingSave, flushPendingSave, hasPendingSave, scheduleSave } from "./autosave";
import { refreshEntries } from "./project";

const currentProject = (): string | null => useProjectStore.getState().project;

/** Apply an edit to the open scenario and schedule it to be saved. */
export function editScenario(scenario: Scenario): void {
  useEditorStore.getState().edited(scenario);
  const project = currentProject();
  const { path } = useEditorStore.getState();
  if (project && path) scheduleSave({ project, path, scenario });
}

export async function selectScenario(path: ScenarioPath): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await flushPendingSave();
  await reportErrors(async () => {
    const [scenario, history] = await Promise.all([
      backend.loadScenario(project, path),
      backend.loadHistory(project, path),
    ]);
    useEditorStore.getState().opened(path, scenario);
    useResponseStore.getState().showHistory(history);
  }, path);
}

/** Ask for a title, then create and open a scenario in `dir`. */
export function createScenarioIn(dir: string): void {
  const project = currentProject();
  if (!project) return;
  useUiStore.getState().askPrompt({
    title: dir ? `New scenario in ${dir}/` : "New scenario",
    placeholder: "Get user",
    initial: "",
    onSubmit: (title) =>
      void reportErrors(async () => {
        const created = await backend.createScenario(project, dir, title);
        await refreshEntries();
        await selectScenario(created);
      }),
  });
}

/** Ask for a new path; a `/` moves the file into a folder. */
export function renameScenario(from: ScenarioPath): void {
  const project = currentProject();
  if (!project) return;
  useUiStore.getState().askPrompt({
    title: "Rename file",
    description: "Path relative to the project. Use a / to move it into a folder.",
    initial: from,
    onSubmit: async (input) => {
      if (!input) return;
      const to = withJsonExtension(input);
      if (to === from) return;
      await flushPendingSave();
      await reportErrors(async () => {
        await backend.renameScenario(project, from, to);
        useEditorStore.getState().moved(from, to);
        await refreshEntries();
      });
    },
  });
}

export async function duplicateScenario(path: ScenarioPath): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await flushPendingSave();
  await reportErrors(async () => {
    const created = await backend.duplicateScenario(project, path);
    await refreshEntries();
    await selectScenario(created);
  });
}

export function deleteScenario(path: ScenarioPath): void {
  const project = currentProject();
  if (!project) return;
  useUiStore.getState().askConfirm({
    title: "Delete scenario?",
    description: `${path} will be deleted from disk, along with its history.`,
    action: "Delete",
    onConfirm: () =>
      void reportErrors(async () => {
        discardPendingSave(path);
        await backend.deleteScenario(project, path);
        if (useEditorStore.getState().path === path) {
          useEditorStore.getState().closed();
          useResponseStore.getState().clearResponse();
        }
        await refreshEntries();
      }),
  });
}

/**
 * Reload the open scenario after an external edit, unless the user has
 * unsaved changes or the file on disk already matches (our own save).
 */
export async function reloadIfChangedOnDisk(changedPaths: string[]): Promise<void> {
  const project = currentProject();
  const { path } = useEditorStore.getState();
  if (!project || !path || !changedPaths.includes(path) || hasPendingSave()) return;
  try {
    const onDisk = await backend.loadScenario(project, path);
    const current = useEditorStore.getState();
    const unchanged = stableStringify(onDisk) === stableStringify(current.scenario);
    if (hasPendingSave() || current.path !== path || unchanged) return;
    current.reloaded(onDisk);
    toast.info("Reloaded: file changed on disk");
  } catch {
    // Deleted or mid-write; the list refresh covers deletion.
  }
}
