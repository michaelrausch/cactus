import { backend, type Scenario, type ProjectPath } from "@/api/client";
import { dirOf } from "@/domain/paths";
import { reportErrors } from "@/lib/errors";
import { isDirty, useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { useUiStore } from "@/stores/uiStore";
import { refreshEntries } from "./project";
import { saveScenario, writingFile } from "./saving";

const currentProject = (): string | null => useProjectStore.getState().project;

/** Apply an edit to the open scenario's buffer. It stays unsaved until the user saves. */
export function editScenario(scenario: Scenario): void {
  useEditorStore.getState().edited(scenario);
}

/** Open a request in the editor. One with unsaved edits shows its buffer, not the file. */
export async function selectScenario(path: ProjectPath): Promise<void> {
  const project = currentProject();
  if (!project) return;
  // Picking a request leaves the Critters view.
  useUiStore.getState().setView("editor");
  if (isDirty(useEditorStore.getState().buffers[path])) {
    useEditorStore.getState().shown(path);
    const history = await reportErrors(() => backend.loadHistory(project, path), path);
    if (history) useResponseStore.getState().historyLoaded(history);
    return;
  }
  await reportErrors(async () => {
    const [scenario, history] = await Promise.all([
      backend.loadScenario(project, path),
      backend.loadHistory(project, path),
    ]);
    useEditorStore.getState().opened(path, scenario);
    useResponseStore.getState().historyLoaded(history);
  }, path);
}

/** Open the New request dialog, creating in `dir` unless another folder is picked. */
export function createScenarioIn(dir: string): void {
  if (!currentProject()) return;
  useUiStore.getState().setNewRequestDir(dir);
}

/** Create a request and open it. Resolves to whether it was created. */
export async function createRequest(dir: string, title: string, method: string, url: string): Promise<boolean> {
  const project = currentProject();
  if (!project) return false;
  const created = await reportErrors(async () => {
    const path = await backend.createScenario(project, dir, title, method, url);
    await refreshEntries();
    await selectScenario(path);
    return true;
  }, "Couldn't create the request");
  return created ?? false;
}

const titleOf = (path: ProjectPath): string =>
  useEditorStore.getState().buffers[path]?.scenario.info.name ||
  useProjectStore.getState().entries.find((entry) => entry.relPath === path)?.title ||
  "";

/** Ask for a new title. The file is renamed to match, staying in its folder. */
export function renameScenario(from: ProjectPath): void {
  const project = currentProject();
  if (!project) return;
  const current = titleOf(from);
  useUiStore.getState().askPrompt({
    title: "Rename request",
    initial: current,
    onSubmit: async (title) => {
      if (!title) return false;
      if (title === current) return true;
      const renamed = await writingFile(from, () =>
        reportErrors(async () => {
          const to = await backend.retitleScenario(project, from, title);
          const editor = useEditorStore.getState();
          if (editor.buffers[from]) {
            editor.moved(from, to);
            editor.retitled(to, await backend.loadScenario(project, to));
          }
          await refreshEntries();
          return true;
        }, "Rename failed"),
      );
      return renamed ?? false;
    },
  });
}

/** Move a request into folder `dir` (`""` for the project root), keeping its file name. */
export async function moveScenario(from: ProjectPath, dir: string): Promise<void> {
  const project = currentProject();
  if (!project || dirOf(from) === dir) return;
  const name = from.split("/").at(-1) ?? from;
  const to = (dir ? `${dir}/${name}` : name) as ProjectPath;
  await writingFile(from, () =>
    reportErrors(async () => {
      await backend.renameScenario(project, from, to);
      useEditorStore.getState().moved(from, to);
      await refreshEntries();
    }, "Move failed"),
  );
}

/** Ask for a name and make a folder, inside `parent` if given. */
export function createFolder(parent = ""): void {
  const project = currentProject();
  if (!project) return;
  useUiStore.getState().askPrompt({
    title: parent ? `New folder in ${parent}/` : "New folder",
    placeholder: "users",
    initial: "",
    onSubmit: async (name) => {
      if (!name) return false;
      const created = await reportErrors(async () => {
        await backend.createFolder(project, parent ? `${parent}/${name}` : name);
        await refreshEntries();
        return true;
      }, "Couldn't create the folder");
      return created ?? false;
    },
  });
}

async function duplicateSaved(project: string, path: ProjectPath): Promise<void> {
  await reportErrors(async () => {
    const created = await backend.duplicateScenario(project, path);
    await refreshEntries();
    await selectScenario(created);
  });
}

/** Duplicates the file on disk, so unsaved edits have to be saved first. */
export async function duplicateScenario(path: ProjectPath): Promise<void> {
  const project = currentProject();
  if (!project) return;
  if (!isDirty(useEditorStore.getState().buffers[path])) return duplicateSaved(project, path);
  useUiStore.getState().askConfirm({
    title: "Save before duplicating?",
    description: `${path} has unsaved changes, which a copy of the file wouldn't include.`,
    action: "Save and duplicate",
    onConfirm: async () => {
      if (await saveScenario(path)) await duplicateSaved(project, path);
    },
  });
}

export function deleteScenario(path: ProjectPath): void {
  const project = currentProject();
  if (!project) return;
  useUiStore.getState().askConfirm({
    title: "Delete request?",
    destructive: true,
    description: `${path} will be deleted from disk, along with its history.`,
    action: "Delete",
    onConfirm: () =>
      void reportErrors(async () => {
        await backend.deleteScenario(project, path);
        if (useEditorStore.getState().path === path) useResponseStore.getState().cleared();
        useEditorStore.getState().removed(path);
        await refreshEntries();
      }),
  });
}

/** Close the editor for `path`, throwing away its unsaved edits. */
export function discardChanges(path: ProjectPath): void {
  if (useEditorStore.getState().path === path) useResponseStore.getState().cleared();
  useEditorStore.getState().removed(path);
  void reportErrors(refreshEntries);
}
