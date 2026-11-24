import { useMemo } from "react";
import { requestVariableNames } from "@/actions/variables";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";

/**
 * Every variable the open scenario can use: the env layers' as the backend
 * resolves them, plus the request's own, which may not be saved yet. Null
 * while the backend hasn't said.
 */
export function useKnownVariables(): string[] | null {
  const names = useProjectStore((s) => s.variableNames);
  const variables = useEditorStore((s) => s.scenario?.runtime?.variables);
  return useMemo(() => {
    if (names === null) return null;
    const own = requestVariableNames(variables);
    return [...new Set([...names, ...own])].toSorted((a, b) => a.localeCompare(b));
  }, [names, variables]);
}
