import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { backend } from "@/api/client";
import { reportErrors } from "@/lib/errors";
import { useCritterStore } from "@/stores/critterStore";
import { useEditorStore } from "@/stores/editorStore";
import { useFlowStore } from "@/stores/flowStore";
import { useMonitorStore } from "@/stores/monitorStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { useSecretsStore } from "@/stores/secretsStore";
import { useUiStore } from "@/stores/uiStore";
import { refreshCritters } from "./critters";
import { refreshSchedules } from "./monitors";
import { saveAll, hasUnsavedChanges } from "./saving";
import { refreshSecrets } from "./secrets";

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
    description: "Some requests have unsaved edits, which opening another folder would lose.",
    action: "Save all and open",
    onConfirm: async () => {
      if (await saveAll()) await loadProject(folder);
    },
    alternative: { action: "Discard changes", onConfirm: () => loadProject(folder), destructive: true },
  });
}

async function loadProject(folder: string): Promise<void> {
  await reportErrors(async () => {
    const entries = await backend.openProject(folder);
    useProjectStore.getState().opened(folder, entries);
    useEditorStore.getState().reset();
    useResponseStore.getState().reset();
    useCritterStore.getState().reset();
    useMonitorStore.getState().reset();
    useFlowStore.getState().closed();
    useSecretsStore.getState().reset();
    useUiStore.getState().setView("editor");
    const [, , , , recent] = await Promise.all([
      refreshEnvs(),
      refreshCritters(),
      refreshSchedules(),
      refreshSecrets(),
      backend.recentProjects(),
    ]);
    useProjectStore.getState().recentProjectsLoaded(recent);
  });
}

/** Reopen the most recently used project on launch. */
export async function restoreLastProject(): Promise<void> {
  const recent = await backend.recentProjects();
  useProjectStore.getState().recentProjectsLoaded(recent);
  if (recent[0]) await openProject(recent[0]);
}

export async function refreshEntries(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const entries = await backend.listScenarios(project);
  // The project may have been switched while listing; don't show the old one's scenarios.
  if (currentProject() === project) useProjectStore.getState().entriesListed(entries);
}

export async function refreshEnvs(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const [envs, activeEnv] = await Promise.all([backend.listEnvs(project), backend.activeEnv(project)]);
  if (currentProject() !== project) return;
  useProjectStore.getState().envsLoaded(envs, activeEnv);
}

export async function selectEnv(name: string | null): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await reportErrors(async () => {
    await backend.setActiveEnv(project, name);
    useProjectStore.getState().envSelected(name);
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
    toast.success(`Imported ${summary.created.length} requests from ${summary.format}`, {
      description: summary.envVars.length ? `Added to opencollection.yml: ${summary.envVars.join(", ")}` : undefined,
    });
  });
}
