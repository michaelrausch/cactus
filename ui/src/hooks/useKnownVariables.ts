import { useMemo } from "react";
import type { Scenario } from "@/api/client";
import { ownVariableNames } from "@/domain/scenario";
import { useProjectStore } from "@/stores/projectStore";

/**
 * Every variable `scenario` can use: the env layers' as the backend resolves
 * them, plus the request's own, which may not be saved yet. Null while the
 * backend hasn't said.
 */
export function useKnownVariables(scenario: Scenario): string[] | null {
  const names = useProjectStore((s) => s.variableNames);
  const variables = scenario.runtime?.variables;
  return useMemo(() => {
    if (names === null) return null;
    const own = ownVariableNames(variables);
    return [...new Set([...names, ...own])].toSorted((a, b) => a.localeCompare(b));
  }, [names, variables]);
}
