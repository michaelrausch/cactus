import { create } from "zustand";
import type { Scenario, ProjectPath } from "@/api/client";
import { stableStringify } from "@/lib/format";

/** How the file on disk disagrees with an unsaved buffer. */
export type Conflict = "changed" | "deleted";

/** A scenario held in memory, with the last content known to be on disk. */
export interface Buffer {
  scenario: Scenario;
  /** What the file held when last loaded or saved. */
  baseline: Scenario;
  /** Set when the file changed on disk while this buffer had unsaved edits. */
  conflict: Conflict | null;
}

/**
 * Scenarios are immutable, so each one's canonical text is worked out once.
 * Dirty checks run in store selectors on every update, so this matters.
 */
const canonical = new WeakMap<Scenario, string>();
function canonicalText(scenario: Scenario): string {
  let text = canonical.get(scenario);
  if (text === undefined) {
    text = stableStringify(scenario);
    canonical.set(scenario, text);
  }
  return text;
}

export const sameScenario = (a: Scenario, b: Scenario): boolean => a === b || canonicalText(a) === canonicalText(b);

/** Whether the buffer differs from disk. Undoing an edit by hand makes it clean again. */
export const isDirty = (buffer: Buffer | undefined): boolean =>
  !!buffer && buffer.scenario !== buffer.baseline && !sameScenario(buffer.scenario, buffer.baseline);

export const dirtyPaths = (buffers: Record<ProjectPath, Buffer>): ProjectPath[] =>
  (Object.keys(buffers) as ProjectPath[]).filter((path) => isDirty(buffers[path]));

interface EditorState {
  /** The scenario being shown, if any. */
  path: ProjectPath | null;
  /** Shortcut for `buffers[path].scenario`. */
  scenario: Scenario | null;
  /** The open scenario, plus every other one with unsaved edits. */
  buffers: Record<ProjectPath, Buffer>;
  /**
   * Bumped whenever the editor must discard its local text state and remount,
   * e.g. on switching scenario or reloading from disk.
   */
  editorVersion: number;

  /** Show `path` with fresh content from disk. */
  opened: (path: ProjectPath, scenario: Scenario) => void;
  /** Show the buffer already held for `path`. */
  shown: (path: ProjectPath) => void;
  edited: (scenario: Scenario) => void;
  /** `scenario` was written; `onDisk` is how it reads back, if that differs. */
  saved: (path: ProjectPath, scenario: Scenario, onDisk?: Scenario) => void;
  /** Replace the buffer with what is on disk. */
  reloaded: (path: ProjectPath, scenario: Scenario) => void;
  conflicted: (path: ProjectPath, conflict: Conflict) => void;
  moved: (from: ProjectPath, to: ProjectPath) => void;
  /** Drop the buffer, closing the editor if it was showing. */
  removed: (path: ProjectPath) => void;
  /** Drop everything, e.g. when switching project. */
  reset: () => void;
}

/** Buffers worth keeping: the one in view and any with unsaved edits. */
function keep(buffers: Record<ProjectPath, Buffer>, current: ProjectPath): Record<ProjectPath, Buffer> {
  const kept: Record<ProjectPath, Buffer> = {};
  for (const path of Object.keys(buffers) as ProjectPath[]) {
    const buffer = buffers[path];
    if (buffer && (path === current || isDirty(buffer))) kept[path] = buffer;
  }
  return kept;
}

const withBuffer = (
  buffers: Record<ProjectPath, Buffer>,
  path: ProjectPath,
  change: (buffer: Buffer) => Buffer,
): Record<ProjectPath, Buffer> => {
  const buffer = buffers[path];
  return buffer ? { ...buffers, [path]: change(buffer) } : buffers;
};

export const useEditorStore = create<EditorState>()((set) => ({
  path: null,
  scenario: null,
  buffers: {},
  editorVersion: 0,

  opened: (path, scenario) =>
    set((s) => ({
      path,
      scenario,
      buffers: keep({ ...s.buffers, [path]: { scenario, baseline: scenario, conflict: null } }, path),
      editorVersion: s.editorVersion + 1,
    })),
  shown: (path) =>
    set((s) => {
      const buffer = s.buffers[path];
      if (!buffer) return {};
      return { path, scenario: buffer.scenario, buffers: keep(s.buffers, path), editorVersion: s.editorVersion + 1 };
    }),
  edited: (scenario) =>
    set((s) =>
      s.path ? { scenario, buffers: withBuffer(s.buffers, s.path, (buffer) => ({ ...buffer, scenario })) } : {},
    ),
  saved: (path, scenario, onDisk = scenario) =>
    set((s) => {
      const buffer = s.buffers[path];
      if (!buffer) return {};
      // Untouched since the save: show exactly what is on disk, so it reads as clean.
      const untouched = buffer.scenario === scenario;
      const shown = untouched ? onDisk : buffer.scenario;
      return {
        buffers: { ...s.buffers, [path]: { scenario: shown, baseline: onDisk, conflict: null } },
        ...(s.path === path ? { scenario: shown } : {}),
      };
    }),
  reloaded: (path, scenario) =>
    set((s) => {
      if (!s.buffers[path]) return {};
      const buffers = { ...s.buffers, [path]: { scenario, baseline: scenario, conflict: null } };
      return s.path === path
        ? { buffers, scenario, editorVersion: s.editorVersion + 1 }
        : { buffers: keep(buffers, s.path ?? path) };
    }),
  conflicted: (path, conflict) =>
    set((s) => ({ buffers: withBuffer(s.buffers, path, (buffer) => ({ ...buffer, conflict })) })),
  moved: (from, to) =>
    set((s) => {
      const buffer = s.buffers[from];
      const { [from]: _gone, ...rest } = s.buffers;
      return {
        buffers: buffer ? { ...rest, [to]: buffer } : s.buffers,
        ...(s.path === from ? { path: to } : {}),
      };
    }),
  removed: (path) =>
    set((s) => {
      const { [path]: _gone, ...buffers } = s.buffers;
      return s.path === path ? { buffers, path: null, scenario: null } : { buffers };
    }),
  reset: () => set({ path: null, scenario: null, buffers: {} }),
}));
