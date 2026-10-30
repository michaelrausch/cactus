import { create } from "zustand";
import type { Scenario, ScenarioEntry, ScenarioPath } from "@/api/client";

interface ProjectState {
  /** Absolute path of the open project folder. */
  project: string | null;
  entries: ScenarioEntry[];
  recentProjects: string[];
  /** Names of the `env.<name>.json` files. */
  envs: string[];
  activeEnv: string | null;

  opened: (project: string, entries: ScenarioEntry[]) => void;
  setEntries: (entries: ScenarioEntry[]) => void;
  setRecentProjects: (recent: string[]) => void;
  setEnvs: (envs: string[], activeEnv: string | null) => void;
  setActiveEnv: (name: string | null) => void;
  /** Reflect a saved scenario in the sidebar without re-listing. */
  entrySaved: (path: ScenarioPath, scenario: Scenario) => void;
}

export const useProjectStore = create<ProjectState>()((set) => ({
  project: null,
  entries: [],
  recentProjects: [],
  envs: [],
  activeEnv: null,

  opened: (project, entries) => set({ project, entries }),
  setEntries: (entries) => set({ entries }),
  setRecentProjects: (recentProjects) => set({ recentProjects }),
  setEnvs: (envs, activeEnv) => set({ envs, activeEnv }),
  setActiveEnv: (activeEnv) => set({ activeEnv }),
  entrySaved: (path, scenario) =>
    set((state) => ({
      entries: state.entries.map((entry) =>
        entry.relPath === path
          ? { ...entry, title: scenario.info.name || entry.title, method: scenario.http.method }
          : entry,
      ),
    })),
}));
