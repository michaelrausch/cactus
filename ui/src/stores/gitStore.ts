import { create } from "zustand";
import type { BranchInfo, DiffSide, FileDiff, GitStatus, ProjectPath } from "@/api/client";

interface GitState {
  /** The open project's changes, or null when it isn't in a readable git repository. */
  status: GitStatus | null;
  /** The project is in no git repository, so it can be set up as one. */
  noRepo: boolean;
  /** Why the repository couldn't be read. */
  error: string | null;
  /** The file and side the diff pane shows. */
  selected: { path: ProjectPath; side: DiffSide } | null;
  /** The diff of `selected`, or null until loaded. */
  diff: FileDiff | null;
  /** Local branches, loaded when the branch menu opens. */
  branches: BranchInfo[];

  /** A status from the backend; null means the project is in no repository. */
  statusLoaded: (status: GitStatus | null) => void;
  statusFailed: (error: string) => void;
  selectionChanged: (selected: { path: ProjectPath; side: DiffSide } | null) => void;
  diffLoaded: (diff: FileDiff | null) => void;
  branchesLoaded: (branches: BranchInfo[]) => void;
  reset: () => void;
}

const EMPTY = { status: null, noRepo: false, error: null, selected: null, diff: null, branches: [] };

export const useGitStore = create<GitState>()((set) => ({
  ...EMPTY,

  statusLoaded: (status) => set({ status, noRepo: status === null, error: null }),
  statusFailed: (error) => set({ status: null, noRepo: false, error }),
  selectionChanged: (selected) => set({ selected }),
  diffLoaded: (diff) => set({ diff }),
  branchesLoaded: (branches) => set({ branches }),
  reset: () => set(EMPTY),
}));
