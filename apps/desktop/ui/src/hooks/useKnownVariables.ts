import { useMemo } from "react";
import type { Scenario, VarSource } from "@/api/client";
import { ownVariableNames } from "@/domain/scenario";
import { sourceDetails } from "@/domain/varSources";
import { selectActiveCritter, useCritterStore } from "@/stores/critterStore";
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

/** Layers a request's own variables override; secrets, captured and critter values override them. */
const BELOW_REQUEST: ReadonlySet<VarSource["kind"]> = new Set(["collection", "environment", "folder"]);

/**
 * Where each variable `scenario` can use gets its value, as the template
 * fields show it: `"staging"`, `"Bob's context · auth/login.yml, 2m ago"`.
 */
export function useVariableDetails(scenario: Scenario): Record<string, string> {
  const sources = useProjectStore((s) => s.variableSources);
  const loadedAt = useProjectStore((s) => s.variableSourcesAt);
  const critter = useCritterStore(selectActiveCritter)?.name;
  const variables = scenario.runtime?.variables;
  return useMemo(() => {
    const merged: Record<string, VarSource> = { ...sources };
    for (const name of ownVariableNames(variables)) {
      const lower = merged[name];
      if (!lower || BELOW_REQUEST.has(lower.kind)) merged[name] = { kind: "request" };
    }
    // Ages are as of loading; sources reload whenever a capture changes them.
    return sourceDetails(merged, critter, loadedAt);
  }, [sources, loadedAt, critter, variables]);
}
