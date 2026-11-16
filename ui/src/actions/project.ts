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
import { useSettingsStore } from "@/stores/settingsStore";
import { useUiStore } from "@/stores/uiStore";
import { refreshCritters } from "./critters";
import { refreshSchedules } from "./monitors";
import { saveAll, hasUnsavedChanges } from "./saving";
import { unlockOnOpen } from "./vault";

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
    title: "Save changes?",
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
    useUiStore.getState().setView("editor");
    await unlockOnOpen(folder);
    const [, , , , recent] = await Promise.all([
      refreshFolders(),
      refreshEnvs(),
      refreshCritters(),
      refreshSchedules(),
      backend.recentProjects(),
    ]);
    useProjectStore.getState().recentProjectsLoaded(recent);
  });
}

/**
 * Reopen the most recently used project on launch, or start the onboarding
 * on a first launch. Anyone with a recent project has used the app already.
 */
export async function restoreLastProject(): Promise<void> {
  const recent = await backend.recentProjects();
  useProjectStore.getState().recentProjectsLoaded(recent);
  const settings = useSettingsStore.getState();
  if (recent[0]) {
    if (!settings.onboarded) settings.setOnboarded(true);
    await openProject(recent[0]);
  } else if (!settings.onboarded) {
    useUiStore.getState().setOnboardingOpen(true);
  }
}

/** Run the onboarding again, from Settings. */
export function showOnboarding(): void {
  const ui = useUiStore.getState();
  ui.setSettingsOpen(false);
  ui.setOnboardingOpen(true);
}

/** Close the onboarding for good. */
export function finishOnboarding(): void {
  useSettingsStore.getState().setOnboarded(true);
  useUiStore.getState().setOnboardingOpen(false);
}

export async function refreshEntries(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const [entries, folders] = await Promise.all([backend.listScenarios(project), backend.listFolders(project)]);
  // The project may have been switched while listing; don't show the old one's scenarios.
  if (currentProject() !== project) return;
  useProjectStore.getState().entriesListed(entries);
  useProjectStore.getState().foldersListed(folders);
}

async function refreshFolders(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const folders = await backend.listFolders(project);
  if (currentProject() === project) useProjectStore.getState().foldersListed(folders);
}

export async function refreshEnvs(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const [envs, activeEnv] = await Promise.all([backend.listEnvs(project), backend.activeEnv(project)]);
  if (currentProject() !== project) return;
  useProjectStore.getState().envsLoaded(envs, activeEnv);
}

/** Make `name` the active env; `null` for none. */
export async function selectEnv(name: string | null): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await reportErrors(async () => {
    await backend.setActiveEnv(project, name);
    useProjectStore.getState().envSelected(name);
  });
}

/** Pick a Postman or OpenAPI file and import it into the open project. */
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
