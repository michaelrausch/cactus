import { toast } from "sonner";
import { backend, type CookieKey, type Critter, type Env, type Species } from "@/api/client";
import { uniqueCritterId } from "@/domain/critterId";
import { flowRunSummary } from "@/domain/flowRun";
import { overriddenNames } from "@/domain/varSources";
import { reportErrors, succeeded, warnQuietly } from "@/lib/errors";
import { useCritterStore } from "@/stores/critterStore";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import { track } from "./achievements";
import { refreshSchedules } from "./monitors";
import { refreshVariableNames } from "./variables";

const currentProject = (): string | null => useProjectStore.getState().project;

/** How long Undo is offered after clearing captured values. */
const UNDO_MS = 6000;

/** List the project's critters, which one is active, and their captured values. */
export async function refreshCritters(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const [critters, activeId] = await Promise.all([backend.listCritters(project), backend.activeCritter(project)]);
  // The project may have been switched while listing.
  if (currentProject() !== project) return;
  useCritterStore.getState().listed(critters, activeId);
  await Promise.all(critters.map((critter) => refreshContext(critter.id)));
}

/** Reload one critter's captured values and cookies. Only a hint (expiry, the lists) if it fails. */
export async function refreshContext(id: string): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const [context, cookies] = await Promise.all([
    warnQuietly(() => backend.loadContext(project, id), `Couldn't load ${id}'s context`),
    warnQuietly(() => backend.loadCookies(project, id), `Couldn't load ${id}'s cookies`),
  ]);
  if (currentProject() !== project) return;
  if (context) useCritterStore.getState().contextLoaded(id, context);
  if (cookies) useCritterStore.getState().cookiesLoaded(id, cookies);
}

/** Remove the given cookies from the critter's jar, or all of them when `cookies` is null. */
export async function removeCookies(id: string, cookies: CookieKey[] | null): Promise<void> {
  const project = currentProject();
  if (!project) return;
  if (await succeeded(() => backend.removeCookies(project, id, cookies), "Couldn't remove cookies")) {
    await refreshContext(id);
  }
}

/** Forget the named captured values (all when `names` is null), offering Undo for a few seconds. */
export async function clearContext(id: string, names: string[] | null): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const removed = await reportErrors(() => backend.clearContext(project, id, names), "Couldn't clear");
  if (!removed) return;
  await refreshContext(id);
  const count = Object.keys(removed).length;
  if (count === 0) return;
  toast(`Cleared ${count === 1 ? Object.keys(removed)[0] : `${count} values`}`, {
    duration: UNDO_MS,
    action: {
      label: "Undo",
      onClick: () =>
        void reportErrors(async () => {
          await backend.restoreContext(project, id, removed);
          await refreshContext(id);
        }, "Couldn't undo"),
    },
  });
}

/** Change `critter.yml` with `change`, editing the file in place. A key `change` leaves out is removed. */
export async function updateCritter(id: string, change: (critter: Critter) => Critter): Promise<boolean> {
  const project = currentProject();
  if (!project) return false;
  const saved = await reportErrors(async () => {
    const critter = await backend.loadCritter(project, id);
    await backend.saveCritter(project, id, change(critter));
    return true;
  }, "Couldn't save the critter");
  if (saved) await reportErrors(refreshCritters);
  return saved ?? false;
}

/** Fill in missing secrets, keeping the critter's other secrets. */
export async function saveCritterSecrets(id: string, values: Env): Promise<boolean> {
  const project = currentProject();
  if (!project) return false;
  const file = { kind: "critterSecrets", name: id } as const;
  const saved = await reportErrors(async () => {
    const existing = await backend.loadEnv(project, file);
    await backend.saveEnv(project, file, { ...existing, ...values });
    return true;
  }, "Couldn't save secrets");
  if (saved) await reportErrors(refreshCritters);
  return saved ?? false;
}

/** Ask for a new folder name (id) and move the critter there. */
export function renameCritterFolder(id: string): void {
  const project = currentProject();
  if (!project) return;
  useUiStore.getState().askPrompt({
    title: "Rename folder",
    description: "Letters, digits, - and _ only. Its schedules and local files move with it.",
    initial: id,
    onSubmit: async (to) => {
      if (!to) return false;
      if (to === id) return true;
      const renamed = await reportErrors(async () => {
        await backend.renameCritter(project, id, to);
        return true;
      }, "Rename failed");
      if (!renamed) return false;
      await reportErrors(refreshCritters);
      useCritterStore.getState().select(to);
      void refreshSchedules();
      return true;
    },
  });
}

/** Ask for a name and copy the critter: its variables and flows, not its secrets or context. */
export function duplicateCritter(id: string): void {
  const project = currentProject();
  const original = useCritterStore.getState().critters.find((critter) => critter.id === id);
  if (!project || !original) return;
  useUiStore.getState().askPrompt({
    title: `Duplicate ${original.name}`,
    initial: `${original.name} copy`,
    onSubmit: async (name) => {
      if (!name) return false;
      const to = uniqueCritterId(
        name,
        useCritterStore.getState().critters.map((critter) => critter.id),
      );
      const copied = await reportErrors(async () => {
        await backend.duplicateCritter(project, id, to, name, original.species);
        return true;
      }, "Couldn't duplicate");
      if (!copied) return false;
      await reportErrors(refreshCritters);
      showCritter(to);
      return true;
    },
  });
}

/** Ask, then delete the critter's folder and local files. The one thing that asks first. */
export function deleteCritter(id: string): void {
  const project = currentProject();
  const critter = useCritterStore.getState().critters.find((c) => c.id === id);
  if (!project || !critter) return;
  useUiStore.getState().askConfirm({
    title: `Delete ${critter.name}?`,
    description: `critters/${id}/ and its flows will be deleted from disk, along with its local secrets, captured values and run logs.`,
    action: "Delete",
    destructive: true,
    onConfirm: () =>
      void reportErrors(async () => {
        const wasActive = useCritterStore.getState().activeId === id;
        await backend.deleteCritter(project, id);
        await refreshCritters();
        void refreshSchedules();
        if (wasActive) void refreshVariableNames();
      }, "Delete failed"),
  });
}

/**
 * The critter's variable and secret names that a lower layer (the active
 * env, the collection, project secrets) also sets, with that layer's label.
 */
export async function critterOverrides(id: string): Promise<[string, string][]> {
  const project = currentProject();
  if (!project) return [];
  const overrides = await warnQuietly(async () => {
    const [lower, variables, secrets] = await Promise.all([
      backend.baseVariableSources(project),
      backend.loadEnv(project, { kind: "critter", name: id }),
      backend.loadEnv(project, { kind: "critterSecrets", name: id }),
    ]);
    return overriddenNames([...Object.keys(variables), ...Object.keys(secrets)], lower);
  }, "Couldn't compare with the environment");
  return overrides ?? [];
}

/** Run the critter's login flow, e.g. when its token has expired. */
export async function logInAgain(id: string): Promise<void> {
  const login = useCritterStore.getState().critters.find((critter) => critter.id === id)?.login;
  if (login) await runFlowNow(id, login);
}

/** Send every request as `id` from now on, or as nobody. */
export async function actAs(id: string | null): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await reportErrors(async () => {
    await backend.setActiveCritter(project, id);
    useCritterStore.getState().activeChanged(id);
    // Critter variables and context change what `{{name}}` resolves to.
    void refreshVariableNames();
  });
}

/**
 * Ctrl+B: from the request editor to the critter last shown (or the
 * active, or the first), and back again.
 */
export function toggleCrittersView(): void {
  if (!currentProject()) return;
  const { view, setView } = useUiStore.getState();
  if (view !== "editor") {
    setView("editor");
    return;
  }
  const { selectedId, activeId, critters } = useCritterStore.getState();
  const id = selectedId ?? activeId ?? critters[0]?.id;
  if (id) showCritter(id);
}

/** Show the critter `id` in the main pane. */
export function showCritter(id: string): void {
  useCritterStore.getState().select(id);
  useUiStore.getState().setView("critter");
}

export function openNewCritter(): void {
  if (currentProject()) useUiStore.getState().setNewCritterOpen(true);
}

/** What a new critter starts with. */
export type CritterStart =
  | { kind: "empty" }
  /** The variable names of an environment, with their values left empty. */
  | { kind: "env"; env: string }
  /** A copy of another critter's `critter.yml` and flows, but not its secrets or context. */
  | { kind: "duplicate"; from: string };

/**
 * Create a critter called `name` and show it. Resolves to whether it was
 * created; on failure the error is shown and the dialog can stay open.
 */
export async function createCritter(name: string, species: Species, start: CritterStart): Promise<boolean> {
  const project = currentProject();
  if (!project) return false;
  const id = uniqueCritterId(
    name,
    useCritterStore.getState().critters.map((critter) => critter.id),
  );
  const created = await reportErrors(async () => {
    if (start.kind === "duplicate") {
      await backend.duplicateCritter(project, start.from, id, name, species);
    } else {
      const variables =
        start.kind === "env"
          ? Object.keys(await backend.loadEnv(project, { kind: "named", name: start.env })).map((variable) => ({
              name: variable,
              value: "",
            }))
          : [];
      await backend.createCritter(project, id, { name, species, variables });
    }
    return true;
  }, "Couldn't create the critter");
  if (!created) return false;
  track("crittersCreated");
  await reportErrors(refreshCritters);
  showCritter(id);
  return true;
}

/** Run one of a critter's flows from a menu, reporting the outcome in a toast. */
export async function runFlowNow(critter: string, flow: string): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const run = await reportErrors(() => backend.runFlow(project, critter, flow, null), `Couldn't run ${flow}`);
  // `null` when cancelled by another send.
  if (!run) return;
  track("flowsRun");
  if (run.ok) toast.success(`${flow} passed`, { description: flowRunSummary(run) });
  else toast.warning(`${flow} failed`, { description: flowRunSummary(run) });
}
