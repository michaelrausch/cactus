import { create } from "zustand";
import type { BranchInfo, DiffSide, FileDiff, GitStatus, ScenarioPath } from "@/api/client";

interface GitState {
  /** The open project's changes, or null when it isn't in a readable git repository. */
  status: GitStatus | null;
  /** The project is in no git repository, so it can be set up as one. */
  noRepo: boolean;
  /** Why the repository couldn't be read. */
  error: string | null;
  /** The file and side the diff pane shows. */
  selected: { path: ScenarioPath; side: DiffSide } | null;
  /** The diff of `selected`, or null until loaded. */
  diff: FileDiff | null;
  /** Local branches, loaded when the branch menu opens. */
  branches: BranchInfo[];

  /** A status from the backend; null means the project is in no repository. */
  loaded: (status: GitStatus | null) => void;
  failed: (error: string) => void;
  select: (selected: { path: ScenarioPath; side: DiffSide } | null) => void;
  setDiff: (diff: FileDiff | null) => void;
  setBranches: (branches: BranchInfo[]) => void;
  reset: () => void;
}

const EMPTY = { status: null, noRepo: false, error: null, selected: null, diff: null, branches: [] };

export const useGitStore = create<GitState>()((set) => ({
  ...EMPTY,

  loaded: (status) => set({ status, noRepo: status === null, error: null }),
  failed: (error) => set({ status: null, noRepo: false, error }),
  select: (selected) => set({ selected }),
  setDiff: (diff) => set({ diff }),
  setBranches: (branches) => set({ branches }),
  reset: () => set(EMPTY),
}));
