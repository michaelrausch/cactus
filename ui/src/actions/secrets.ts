import { toast } from "sonner";
import { backend } from "@/api/client";
import { errorMessage, reportErrors, warnQuietly } from "@/lib/errors";
import { useProjectStore } from "@/stores/projectStore";
import { useSecretsStore } from "@/stores/secretsStore";

const currentProject = (): string | null => useProjectStore.getState().project;

export async function refreshSecrets(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  if (!import.meta.env.SECRETS_ENCRYPTION) {
    useSecretsStore.getState().loaded({ status: "off", autoLockMinutes: null, hasSealedValues: false });
    return;
  }
  const info = await warnQuietly(() => backend.secretsStatus(project), "Couldn't read secrets status");
  if (info && currentProject() === project) useSecretsStore.getState().loaded(info);
}

/** Callers of `ensureUnlocked` waiting on the unlock dialog. */
let waiting: ((unlocked: boolean) => void)[] = [];

/**
 * Resolves `true` once secrets can be used: at once when nothing needs
 * the key, otherwise after the unlock dialog, or `false` if it's cancelled.
 * Called before anything that sends, so a lock is a dialog rather than an
 * error. Sends need the key only when a value is encrypted; `force` asks
 * whenever locked, e.g. before saving a secret.
 */
export async function ensureUnlocked({ force = false }: { force?: boolean } = {}): Promise<boolean> {
  if (!useSecretsStore.getState().info) await refreshSecrets();
  const info = useSecretsStore.getState().info;
  if (info?.status !== "locked" || (!force && !info.hasSealedValues)) return true;
  useSecretsStore.getState().setDialog("unlock");
  return new Promise((resolve) => waiting.push(resolve));
}

/** The unlock dialog closed, unlocked or not. */
export function unlockDialogClosed(unlocked: boolean): void {
  const resolvers = waiting;
  waiting = [];
  if (useSecretsStore.getState().dialog === "unlock") useSecretsStore.getState().setDialog(null);
  resolvers.forEach((resolve) => resolve(unlocked));
}

/** From the unlock dialog: whatever was waiting is cancelled, and reset opens. */
export function forgotPassword(): void {
  unlockDialogClosed(false);
  useSecretsStore.getState().setDialog("reset");
}

/**
 * Work for a dialog: resolves to `null` when done, or the error message to
 * show in the dialog so the password can be fixed and retried.
 */
async function attempt(work: (project: string) => Promise<unknown>): Promise<string | null> {
  const project = currentProject();
  if (!project) return "No project open";
  try {
    await work(project);
    return null;
  } catch (error) {
    return errorMessage(error);
  } finally {
    await refreshSecrets();
  }
}

// The dialog actions below resolve to an error to show, or null; see `attempt`.

export async function unlockSecrets(password: string): Promise<string | null> {
  return attempt((project) => backend.unlockSecrets(project, password));
}

export async function enableEncryption(password: string, autoLockMinutes: number | null): Promise<string | null> {
  const problem = await attempt((project) => backend.enableSecretsEncryption(project, password, autoLockMinutes));
  if (!problem) toast.success("Secrets encrypted");
  return problem;
}

export async function disableEncryption(password: string): Promise<string | null> {
  const problem = await attempt((project) => backend.disableSecretsEncryption(project, password));
  if (!problem) toast.success("Encryption off");
  return problem;
}

export async function changePassword(current: string, next: string): Promise<string | null> {
  const problem = await attempt((project) => backend.changeSecretsPassword(project, current, next));
  if (!problem) toast.success("Password changed");
  return problem;
}

export async function resetSecrets(): Promise<string | null> {
  const problem = await attempt((project) => backend.resetSecrets(project));
  if (!problem) toast.success("Secrets reset");
  return problem;
}

export async function lockSecrets(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await reportErrors(() => backend.lockSecrets(project));
  await refreshSecrets();
}

export async function setAutoLock(minutes: number | null): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await reportErrors(() => backend.setAutoLock(project, minutes), "Couldn't change auto-lock");
  await refreshSecrets();
}
