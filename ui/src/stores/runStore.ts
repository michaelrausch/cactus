import { create } from "zustand";
import type { RunResult } from "@/api/client";

export interface FolderRun {
  /** Folder being run; `""` for the whole project. */
  dir: string;
  total: number;
  results: RunResult[];
  done: boolean;
}

interface RunState {
  run: FolderRun | null;
  started: (dir: string, total: number) => void;
  resultArrived: (result: RunResult) => void;
  finished: () => void;
  dismissed: () => void;
}

export const useRunStore = create<RunState>()((set) => ({
  run: null,
  started: (dir, total) => set({ run: { dir, total, results: [], done: false } }),
  resultArrived: (result) => set((s) => (s.run ? { run: { ...s.run, results: [...s.run.results, result] } } : {})),
  finished: () => set((s) => (s.run ? { run: { ...s.run, done: true } } : {})),
  dismissed: () => set({ run: null }),
}));
