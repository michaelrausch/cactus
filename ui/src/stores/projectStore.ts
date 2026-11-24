import { create } from "zustand";
import type { Scenario, ScenarioEntry, ProjectPath } from "@/api/client";

interface ProjectState {
  /** Absolute path of the open project folder. */
  project: string | null;
  entries: ScenarioEntry[];
  recentProjects: string[];
  /** Names of the environments, from `environments/<name>.yml`. */
  envs: string[];
  activeEnv: string | null;
  /**
   * Every variable the open scenario can use with the active env, or null
   * until known (nothing is flagged as unresolved then).
   */
  variableNames: string[] | null;

  opened: (project: string, entries: ScenarioEntry[]) => void;
  entriesListed: (entries: ScenarioEntry[]) => void;
  recentProjectsLoaded: (recent: string[]) => void;
  envsLoaded: (envs: string[], activeEnv: string | null) => void;
  envSelected: (name: string | null) => void;
  variableNamesLoaded: (names: string[] | null) => void;
  /** Reflect a saved scenario in the sidebar without re-listing. */
  entrySaved: (path: ProjectPath, scenario: Scenario) => void;
}

export const useProjectStore = create<ProjectState>()((set) => ({
  project: null,
  entries: [],
  recentProjects: [],
  envs: [],
  activeEnv: null,
  variableNames: null,

  opened: (project, entries) => set({ project, entries, variableNames: null }),
  entriesListed: (entries) => set({ entries }),
  recentProjectsLoaded: (recentProjects) => set({ recentProjects }),
  envsLoaded: (envs, activeEnv) => set({ envs, activeEnv }),
  envSelected: (activeEnv) => set({ activeEnv }),
  variableNamesLoaded: (variableNames) => set({ variableNames }),
  entrySaved: (path, scenario) =>
    set((state) => ({
      entries: state.entries.map((entry) =>
        entry.relPath === path
          ? { ...entry, title: scenario.info.name || entry.title, method: scenario.http.method }
          : entry,
      ),
    })),
}));
