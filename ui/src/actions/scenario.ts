import { backend, type Scenario, type ScenarioPath } from "@/api/client";
import { withRequestExtension } from "@/domain/paths";
import { reportErrors } from "@/lib/errors";
import { isDirty, useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { useUiStore } from "@/stores/uiStore";
import { refreshEntries } from "./project";
import { saveScenario } from "./saving";

const currentProject = (): string | null => useProjectStore.getState().project;

/** Apply an edit to the open scenario's buffer. It stays unsaved until the user saves. */
export function editScenario(scenario: Scenario): void {
  useEditorStore.getState().edited(scenario);
}

export async function selectScenario(path: ScenarioPath): Promise<void> {
  const project = currentProject();
  if (!project) return;
  if (isDirty(useEditorStore.getState().buffers[path])) {
    useEditorStore.getState().shown(path);
    useResponseStore.getState().showHistory(await backend.loadHistory(project, path));
    return;
  }
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
    title: dir ? `New request in ${dir}/` : "New request",
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
      const to = withRequestExtension(input);
      if (to === from) return;
      await reportErrors(async () => {
        await backend.renameScenario(project, from, to);
        useEditorStore.getState().moved(from, to);
        await refreshEntries();
      });
    },
  });
}

async function duplicateSaved(project: string, path: ScenarioPath): Promise<void> {
  await reportErrors(async () => {
    const created = await backend.duplicateScenario(project, path);
    await refreshEntries();
    await selectScenario(created);
  });
}

/** Duplicates the file on disk, so unsaved edits have to be saved first. */
export async function duplicateScenario(path: ScenarioPath): Promise<void> {
  const project = currentProject();
  if (!project) return;
  if (!isDirty(useEditorStore.getState().buffers[path])) return duplicateSaved(project, path);
  useUiStore.getState().askConfirm({
    title: "Save before duplicating?",
    description: `${path} has unsaved changes, which a copy of the file wouldn't include.`,
    action: "Save and duplicate",
    destructive: false,
    onConfirm: async () => {
      if (await saveScenario(path)) await duplicateSaved(project, path);
    },
  });
}

export function deleteScenario(path: ScenarioPath): void {
  const project = currentProject();
  if (!project) return;
  useUiStore.getState().askConfirm({
    title: "Delete request?",
    description: `${path} will be deleted from disk, along with its history.`,
    action: "Delete",
    onConfirm: () =>
      void reportErrors(async () => {
        await backend.deleteScenario(project, path);
        if (useEditorStore.getState().path === path) useResponseStore.getState().clearResponse();
        useEditorStore.getState().removed(path);
        await refreshEntries();
      }),
  });
}

/** Close the editor for `path`, throwing away its unsaved edits. */
export function discardChanges(path: ScenarioPath): void {
  if (useEditorStore.getState().path === path) useResponseStore.getState().clearResponse();
  useEditorStore.getState().removed(path);
  void reportErrors(refreshEntries);
}
