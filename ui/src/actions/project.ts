import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { backend } from "@/api/client";
import { reportErrors } from "@/lib/errors";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { useUiStore } from "@/stores/uiStore";
import { saveAll, hasUnsavedChanges } from "./saving";

const IMPORT_FILTERS = [{ name: "Postman / OpenAPI", extensions: ["json", "yaml", "yml"] }];

const currentProject = (): string | null => useProjectStore.getState().project;

async function pickFolder(): Promise<string | null> {
  const choice = await open({ directory: true });
  return typeof choice === "string" ? choice : null;
}

/** Open `path`, or ask for a folder when none is given. */
export async function openProject(path?: string): Promise<void> {
  const folder = path ?? (await pickFolder());
  if (!folder) return;
  if (!hasUnsavedChanges()) return loadProject(folder);
  useUiStore.getState().askConfirm({
    title: "Unsaved changes",
    description: "Some scenarios have unsaved edits, which opening another folder would lose.",
    action: "Save all and open",
    destructive: false,
    onConfirm: async () => {
      if (await saveAll()) await loadProject(folder);
    },
    alternative: { action: "Discard changes", onConfirm: () => loadProject(folder) },
  });
}

async function loadProject(folder: string): Promise<void> {
  await reportErrors(async () => {
    const entries = await backend.openProject(folder);
    useProjectStore.getState().opened(folder, entries);
    useEditorStore.getState().reset();
    useResponseStore.getState().reset();
    const [, recent] = await Promise.all([refreshEnvs(), backend.recentProjects()]);
    useProjectStore.getState().setRecentProjects(recent);
  });
}

/** Reopen the most recently used project on launch. */
export async function restoreLastProject(): Promise<void> {
  const recent = await backend.recentProjects();
  useProjectStore.getState().setRecentProjects(recent);
  if (recent[0]) await openProject(recent[0]);
}

export async function refreshEntries(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const entries = await backend.listScenarios(project);
  // The project may have been switched while listing; don't show the old one's scenarios.
  if (currentProject() === project) useProjectStore.getState().setEntries(entries);
}

export async function refreshEnvs(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const [envs, activeEnv] = await Promise.all([backend.listEnvs(project), backend.activeEnv(project)]);
  if (currentProject() !== project) return;
  useProjectStore.getState().setEnvs(envs, activeEnv);
}

export async function selectEnv(name: string | null): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await reportErrors(async () => {
    await backend.setActiveEnv(project, name);
    useProjectStore.getState().setActiveEnv(name);
  });
}

export async function importCollection(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const file = await open({ filters: IMPORT_FILTERS });
  if (typeof file !== "string") return;
  await reportErrors(async () => {
    const summary = await backend.importFile(project, file);
    await refreshEntries();
    await refreshEnvs();
    toast.success(`Imported ${summary.created.length} scenarios from ${summary.format}`, {
      description: summary.envVars.length ? `Added to env.json: ${summary.envVars.join(", ")}` : undefined,
    });
  });
}
