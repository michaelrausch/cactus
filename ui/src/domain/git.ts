import type { ChangeKind, DiffSide, FileStatus, GitStatus, ProjectPath } from "@/api/client";
import { baseName, dirOf } from "./paths";

/** A file's staged or unstaged changes, as shown in the diff pane. */
export interface DiffSelection {
  path: ProjectPath;
  side: DiffSide;
}

const MARKERS: Record<ChangeKind, string> = {
  added: "A",
  modified: "M",
  deleted: "D",
  renamed: "R",
  untracked: "U",
  conflicted: "!",
};

const LABELS: Record<ChangeKind, string> = {
  added: "Added",
  modified: "Modified",
  deleted: "Deleted",
  renamed: "Renamed",
  untracked: "Untracked",
  conflicted: "Conflicted",
};

/** One letter, as `git status --short` shows it. */
export const markerFor = (kind: ChangeKind): string => MARKERS[kind];

export const kindLabel = (kind: ChangeKind): string => LABELS[kind];

export const stagedFiles = (status: GitStatus): FileStatus[] => status.files.filter((file) => file.staged);

export const unstagedFiles = (status: GitStatus): FileStatus[] => status.files.filter((file) => file.unstaged);

export const changeOn = (file: FileStatus, side: DiffSide): ChangeKind | null =>
  side === "staged" ? file.staged : file.unstaged;

export const isUntracked = (file: FileStatus): boolean => file.unstaged === "untracked";

/** Unstaging a rename has to put its old path back too. */
export const unstagePaths = (file: FileStatus): ProjectPath[] =>
  file.renamedFrom ? [file.path, file.renamedFrom] : [file.path];

/** The change the sidebar shows per file: unstaged work first, as it is the newest. */
export function changesByPath(status: GitStatus | null): Record<ProjectPath, ChangeKind> {
  const changes: Record<ProjectPath, ChangeKind> = {};
  for (const file of status?.files ?? []) {
    const kind = file.unstaged ?? file.staged;
    if (kind) changes[file.path] = kind;
  }
  return changes;
}

export const sameSelection = (a: DiffSelection | null, b: DiffSelection | null): boolean =>
  a?.path === b?.path && a?.side === b?.side;

/**
 * What to show after the status changes: the same file and side if it still
 * has changes there, else the file's other side (it was just staged or
 * unstaged), else the first change listed.
 */
export function nextSelection(status: GitStatus, selected: DiffSelection | null): DiffSelection | null {
  const file = selected && status.files.find((f) => f.path === selected.path);
  if (file && selected) {
    if (changeOn(file, selected.side)) return selected;
    const other: DiffSide = selected.side === "staged" ? "unstaged" : "staged";
    if (changeOn(file, other)) return { path: file.path, side: other };
  }
  const [firstStaged] = stagedFiles(status);
  if (firstStaged) return { path: firstStaged.path, side: "staged" };
  const [firstUnstaged] = unstagedFiles(status);
  return firstUnstaged ? { path: firstUnstaged.path, side: "unstaged" } : null;
}

/**
 * How a changed file is named in the list: a request by its title with its
 * path beside it, any other file by its name with its folder beside it.
 */
export function changeLabel(path: ProjectPath, titles: Record<string, string>): { name: string; detail: string } {
  const title = titles[path];
  return title ? { name: title, detail: path } : { name: baseName(path), detail: dirOf(path) };
}
