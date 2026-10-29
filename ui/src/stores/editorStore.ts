import { create } from "zustand";
import type { Scenario, ScenarioPath } from "@/api/client";

interface EditorState {
  path: ScenarioPath | null;
  scenario: Scenario | null;
  /**
   * Bumped whenever the editor must discard its local text state and remount,
   * e.g. on switching scenario or reloading from disk.
   */
  editorVersion: number;

  opened: (path: ScenarioPath, scenario: Scenario) => void;
  reloaded: (scenario: Scenario) => void;
  edited: (scenario: Scenario) => void;
  moved: (from: ScenarioPath, to: ScenarioPath) => void;
  closed: () => void;
}

export const useEditorStore = create<EditorState>()((set) => ({
  path: null,
  scenario: null,
  editorVersion: 0,

  opened: (path, scenario) => set((s) => ({ path, scenario, editorVersion: s.editorVersion + 1 })),
  reloaded: (scenario) => set((s) => ({ scenario, editorVersion: s.editorVersion + 1 })),
  edited: (scenario) => set({ scenario }),
  moved: (from, to) => set((s) => (s.path === from ? { path: to } : {})),
  closed: () => set({ path: null, scenario: null }),
}));
