import { create } from "zustand";
import type { Scenario, ScenarioEntry, ProjectPath, VarSource } from "@/api/client";

interface ProjectState {
  /** Absolute path of the open project folder. */
  project: string | null;
  entries: ScenarioEntry[];
  /** Request folders, empty ones included. */
  folders: string[];
  recentProjects: string[];
  /** Names of the environments, from `environments/<name>.yml`. */
  envs: string[];
  activeEnv: string | null;
  /**
   * Every variable the open scenario can use with the active env and
   * critter, or null until known (nothing is flagged as unresolved then).
   */
  variableNames: string[] | null;
  /** Where each of `variableNames` gets its value. */
  variableSources: Record<string, VarSource> | null;
  /** When `variableSources` was loaded, so captured values can say how old they are. */
  variableSourcesAt: number;

  opened: (project: string, entries: ScenarioEntry[]) => void;
  entriesListed: (entries: ScenarioEntry[]) => void;
  foldersListed: (folders: string[]) => void;
  recentProjectsLoaded: (recent: string[]) => void;
  envsLoaded: (envs: string[], activeEnv: string | null) => void;
  envSelected: (name: string | null) => void;
  variableSourcesLoaded: (sources: Record<string, VarSource> | null, at: number) => void;
  /** Reflect a saved scenario in the sidebar without re-listing. */
  entrySaved: (path: ProjectPath, scenario: Scenario) => void;
}

export const useProjectStore = create<ProjectState>()((set) => ({
  project: null,
  entries: [],
  folders: [],
  recentProjects: [],
  envs: [],
  activeEnv: null,
  variableNames: null,
  variableSources: null,
  variableSourcesAt: 0,

  opened: (project, entries) => set({ project, entries, folders: [], variableNames: null, variableSources: null }),
  entriesListed: (entries) => set({ entries }),
  foldersListed: (folders) => set({ folders }),
  recentProjectsLoaded: (recentProjects) => set({ recentProjects }),
  envsLoaded: (envs, activeEnv) => set({ envs, activeEnv }),
  envSelected: (activeEnv) => set({ activeEnv }),
  variableSourcesLoaded: (variableSources, variableSourcesAt) =>
    set({ variableSources, variableSourcesAt, variableNames: variableSources ? Object.keys(variableSources) : null }),
  entrySaved: (path, scenario) =>
    set((state) => ({
      entries: state.entries.map((entry) =>
        entry.relPath === path
          ? {
              ...entry,
              title: scenario.info.name || entry.title,
              method: scenario.http.method,
              tags: scenario.info.tags ?? [],
            }
          : entry,
      ),
    })),
}));
