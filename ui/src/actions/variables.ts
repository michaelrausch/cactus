import { backend, type Scenario } from "@/api/client";
import { errorMessage } from "@/lib/errors";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";

/** Only the newest request's answer is kept. */
let latestRequest = 0;

/**
 * Reload the names of the variables the open scenario can use. Call it when
 * the scenario, the active env or any env layer may have changed.
 */
export async function refreshVariableNames(): Promise<void> {
  const { project, setVariableNames } = useProjectStore.getState();
  const { path } = useEditorStore.getState();
  const request = ++latestRequest;
  if (!project || !path) {
    setVariableNames(null);
    return;
  }
  try {
    const names = await backend.variableNames(project, path);
    if (request === latestRequest) setVariableNames(names);
  } catch (error) {
    // Highlighting is a hint; without names nothing is flagged as unresolved.
    console.warn(`Couldn't list variables: ${errorMessage(error)}`);
    if (request === latestRequest) setVariableNames(null);
  }
}

/** The request's own active variables, which may not be saved yet. */
export const requestVariableNames = (variables: NonNullable<Scenario["runtime"]>["variables"]): string[] =>
  (variables ?? []).filter((v) => !v.disabled && v.name.trim()).map((v) => v.name.trim());
