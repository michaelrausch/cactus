import { backend } from "@/api/client";
import { warnQuietly } from "@/lib/errors";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";

/** Only the newest request's answer is kept. */
let latestRequest = 0;

/**
 * Reload the names of the variables the open scenario can use. Call it when
 * the scenario, the active env or any env layer may have changed.
 */
export async function refreshVariableNames(): Promise<void> {
  const { project, variableNamesLoaded } = useProjectStore.getState();
  const { path } = useEditorStore.getState();
  const request = ++latestRequest;
  if (!project || !path) {
    variableNamesLoaded(null);
    return;
  }
  // Highlighting is a hint; without names nothing is flagged as unresolved.
  const names = await warnQuietly(() => backend.variableNames(project, path), "Couldn't list variables");
  if (request === latestRequest) variableNamesLoaded(names ?? null);
}
