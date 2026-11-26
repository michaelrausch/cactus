import { backend, type TokenStatus } from "@/api/client";
import { reportErrors, succeeded, warnQuietly } from "@/lib/errors";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";

/** The open request as edited, and its project. */
function openRequest() {
  const { project } = useProjectStore.getState();
  const { path, scenario } = useEditorStore.getState();
  return project && path && scenario ? { project, path, scenario } : null;
}

/** The open request's cached token, for the active env and critter. Only a hint if it fails. */
export async function loadTokenStatus(): Promise<TokenStatus | undefined> {
  const open = openRequest();
  if (!open) return undefined;
  return warnQuietly(
    () => backend.oauth2TokenStatus(open.project, open.path, open.scenario),
    "Couldn't read the token",
  );
}

/** Get a new token; the authorization code grant goes through the browser. `undefined` when it failed or was cancelled. */
export async function getToken(): Promise<TokenStatus | undefined> {
  const open = openRequest();
  if (!open) return undefined;
  const status = await reportErrors(
    () => backend.getOauth2Token(open.project, open.path, open.scenario),
    "Couldn't get a token",
  );
  return status ?? undefined;
}

/** Stop waiting for the browser. */
export const cancelGetToken = (): Promise<void> => backend.cancelRequest();

export async function clearToken(): Promise<boolean> {
  const open = openRequest();
  if (!open) return false;
  return succeeded(() => backend.clearOauth2Token(open.project, open.path, open.scenario), "Couldn't clear the token");
}
