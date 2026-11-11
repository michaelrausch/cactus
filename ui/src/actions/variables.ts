import { backend } from "@/api/client";
import { warnQuietly } from "@/lib/errors";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";

/** Only the newest request's answer is kept. */
let latestRequest = 0;

/**
 * Reload the variables the open scenario can use and where each comes
 * from. Call it when the scenario, the active env or critter, or any env
 * layer may have changed.
 */
export async function refreshVariableNames(): Promise<void> {
  const { project, variableSourcesLoaded } = useProjectStore.getState();
  const { path } = useEditorStore.getState();
  const request = ++latestRequest;
  if (!project || !path) {
    variableSourcesLoaded(null, Date.now());
    return;
  }
  // Highlighting is a hint; without names nothing is flagged as unresolved.
  const sources = await warnQuietly(() => backend.variableSources(project, path), "Couldn't list variables");
  if (request === latestRequest) variableSourcesLoaded(sources ?? null, Date.now());
}
