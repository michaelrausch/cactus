import type { TotpSetup, VaultStatus } from "@/api/bindings";
import { backend } from "@/api/client";
import { errorMessage } from "@/lib/errors";
import { useProjectStore } from "@/stores/projectStore";
import { useVaultStore } from "@/stores/vaultStore";

/** Resolves the wait in {@link unlockOnOpen} once the project is unlocked or wiped. */
let leftLock: (() => void) | null = null;

/**
 * Load `folder`'s status; when it's locked, wait until it's unlocked or wiped.
 * Opening another project from the lock screen abandons the wait, so the
 * locked project never finishes loading.
 */
export async function unlockOnOpen(folder: string): Promise<void> {
  leftLock = null;
  await refreshVaultStatus(folder);
  if (useVaultStore.getState().status !== "locked") return;
  await new Promise<void>((resolve) => {
    leftLock = resolve;
  });
}

/** Reload `project`'s status, and whether unlocking it takes a code. */
export async function refreshVaultStatus(project: string | null): Promise<void> {
  if (!project) {
    useVaultStore.getState().statusLoaded(null, false);
    return;
  }
  const [status, totp] = await Promise.all([backend.vaultStatus(project), backend.totpEnabled(project)]);
  useVaultStore.getState().statusLoaded(status, totp);
}

export interface UnlockResult {
  error: string | null;
  /** The password was right; ask for the authenticator code. */
  codeRequired: boolean;
}

export async function unlock(password: string, code: string | null): Promise<UnlockResult> {
  const project = currentProject();
  if (!project) return { error: null, codeRequired: false };
  try {
    if ((await backend.unlockVault(project, password, code)) === "codeRequired") {
      return { error: null, codeRequired: true };
    }
  } catch (error) {
    return { error: errorMessage(error), codeRequired: code !== null };
  }
  await leaveLock(project, "unlocked");
  return { error: null, codeRequired: false };
}

/** Delete the project's secrets, context and password. Resolves to an error to show, or null when done. */
export async function wipeSecrets(): Promise<string | null> {
  const project = currentProject();
  if (!project) return null;
  const error = await attempt(() => backend.wipeVault(project));
  if (!error) await leaveLock(project, "noPassword");
  return error;
}

/** Set, change or (with `next` null) remove the password. Resolves to an error to show, or null when done. */
export async function setPassword(current: string | null, next: string | null): Promise<string | null> {
  return changeVault((project) => backend.setVaultPassword(project, current, next));
}

/** A new secret to scan; nothing changes until {@link enableTotp}. */
export async function startTotp(): Promise<TotpSetup | string> {
  const project = currentProject();
  if (!project) return "No project open";
  try {
    return await backend.startTotp(project);
  } catch (error) {
    return errorMessage(error);
  }
}

export function enableTotp(secret: string, code: string): Promise<string | null> {
  return changeVault((project) => backend.enableTotp(project, secret, code));
}

export function disableTotp(code: string): Promise<string | null> {
  return changeVault((project) => backend.disableTotp(project, code));
}

const currentProject = (): string | null => useProjectStore.getState().project;

async function attempt(work: () => Promise<unknown>): Promise<string | null> {
  try {
    await work();
    return null;
  } catch (error) {
    return errorMessage(error);
  }
}

/** Run `work` on the open project, then reload its status. Resolves to an error to show, or null when done. */
async function changeVault(work: (project: string) => Promise<unknown>): Promise<string | null> {
  const project = currentProject();
  if (!project) return null;
  const error = await attempt(() => work(project));
  if (!error) await refreshVaultStatus(project);
  return error;
}

async function leaveLock(project: string, status: VaultStatus): Promise<void> {
  useVaultStore.getState().statusLoaded(status, false);
  await refreshVaultStatus(project).catch(() => undefined);
  leftLock?.();
  leftLock = null;
}
