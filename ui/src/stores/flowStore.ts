import { create } from "zustand";
import type {
  Flow,
  FlowAsked,
  FlowDisplayed,
  FlowRun,
  FlowStepResult,
  FlowStepStart,
  Scenario,
  VarSource,
} from "@/api/client";
import { flowFingerprint } from "@/domain/flowEdit";

/** Undo history kept per open flow. */
const MAX_UNDO = 100;

export interface OpenFlow {
  critter: string;
  flow: string;
}

export type FlowMode = "visual" | "yaml";

export interface ParseProblem {
  message: string;
  line: number | null;
  column: number | null;
}

/** The right-hand pane: the selected step's settings, or the run. */
export type FlowPanel = "step" | "run";

/** For the save button: writing, or just written. */
export type SaveStatus = "idle" | "saving" | "saved";

export interface FlowRunState {
  running: boolean;
  /** Per step, where it is sending, once it starts. */
  started: (FlowStepStart | null)[];
  /** One per step: its last result, or null while queued or never run. */
  results: (FlowStepResult | null)[];
  /** The step running now. */
  current: number | null;
  /** The finished run, for the summary line. */
  last: FlowRun | null;
  /** The selection moves to each step as it starts, until the user picks one. */
  following: boolean;
  /** The flow was edited after this run, so the results may no longer match it. */
  stale: boolean;
}

const NO_RUN: FlowRunState = {
  running: false,
  started: [],
  results: [],
  current: null,
  last: null,
  following: false,
  stale: false,
};

interface FlowState {
  open: OpenFlow | null;
  /** The flow as edited; in YAML mode, the text's last successful parse. */
  model: Flow | null;
  savedFingerprint: string;
  /** The YAML editor's text. */
  text: string;
  savedText: string;
  mode: FlowMode;
  /** Why `text` doesn't parse, while it doesn't. */
  parseProblem: ParseProblem | null;
  undo: Flow[];
  redo: Flow[];
  /** The step shown in the inspector and the run pane. */
  selected: number | null;
  panel: FlowPanel;
  /** Each step's request by path; null when it doesn't exist or doesn't parse. */
  requests: Record<string, Scenario | null>;
  /** What the flow's critter sees before any step runs. */
  scope: Record<string, VarSource>;
  run: FlowRunState;
  /** The file changed on disk while there were unsaved edits. */
  conflict: boolean;
  saveStatus: SaveStatus;
  /**
   * A running flow's question (a prompt, or a paused display) waiting for an
   * answer. Kept when another flow opens: the run still waits for it.
   */
  ask: FlowAsked | null;
  /** A running flow's message (a display step without pause), until dismissed. */
  message: FlowDisplayed | null;

  opened: (open: OpenFlow, model: Flow, text: string, scope: Record<string, VarSource>) => void;
  closed: () => void;
  /** A visual edit: undoable, and it makes the last run's results stale. */
  edited: (model: Flow) => void;
  undone: () => void;
  redone: () => void;
  textEdited: (text: string) => void;
  textParsed: (model: Flow | null, problem: ParseProblem | null) => void;
  modeChanged: (mode: FlowMode, text?: string) => void;
  saved: (model: Flow, text: string) => void;
  /** Saved while the user kept editing: what is on disk is the new baseline, and the edits stay. */
  savedBehindEdits: (model: Flow, text: string) => void;
  select: (index: number | null) => void;
  panelShown: (panel: FlowPanel) => void;
  requestsLoaded: (requests: Record<string, Scenario | null>) => void;
  runStarted: (steps: number) => void;
  stepStarted: (start: FlowStepStart) => void;
  stepFinished: (result: FlowStepResult) => void;
  runFinished: (run: FlowRun | null) => void;
  conflicted: () => void;
  saveStatusChanged: (status: SaveStatus) => void;
  asked: (ask: FlowAsked) => void;
  /** Answered, or no longer waited for. */
  askClosed: (id: number) => void;
  /** A newer message replaces one still showing. */
  displayed: (message: FlowDisplayed | null) => void;
}

const clampSelection = (selected: number | null, model: Flow): number | null =>
  selected === null || model.steps.length === 0 ? null : Math.min(selected, model.steps.length - 1);

export const useFlowStore = create<FlowState>()((set) => ({
  open: null,
  model: null,
  savedFingerprint: "",
  text: "",
  savedText: "",
  mode: "visual",
  parseProblem: null,
  undo: [],
  redo: [],
  selected: null,
  panel: "step",
  requests: {},
  scope: {},
  run: NO_RUN,
  conflict: false,
  saveStatus: "idle",
  ask: null,
  message: null,

  opened: (open, model, text, scope) =>
    set((state) => ({
      open,
      model,
      savedFingerprint: flowFingerprint(model),
      text,
      savedText: text,
      parseProblem: null,
      undo: [],
      redo: [],
      selected: model.steps.length > 0 ? 0 : null,
      panel: "step",
      scope,
      run: NO_RUN,
      conflict: false,
      // Keep the mode and loaded requests when the same flow reloads.
      mode: state.open?.critter === open.critter && state.open.flow === open.flow ? state.mode : "visual",
    })),
  closed: () =>
    set({
      open: null,
      model: null,
      undo: [],
      redo: [],
      selected: null,
      run: NO_RUN,
      conflict: false,
      saveStatus: "idle",
    }),
  edited: (model) =>
    set((state) => ({
      model,
      undo: state.model ? [...state.undo, state.model].slice(-MAX_UNDO) : state.undo,
      redo: [],
      selected: clampSelection(state.selected, model),
      run: state.run.results.length > 0 ? { ...state.run, stale: true } : state.run,
    })),
  undone: () =>
    set((state) => {
      const previous = state.undo.at(-1);
      if (!previous || !state.model) return {};
      return {
        model: previous,
        undo: state.undo.slice(0, -1),
        redo: [...state.redo, state.model],
        selected: clampSelection(state.selected, previous),
      };
    }),
  redone: () =>
    set((state) => {
      const next = state.redo.at(-1);
      if (!next || !state.model) return {};
      return {
        model: next,
        redo: state.redo.slice(0, -1),
        undo: [...state.undo, state.model],
        selected: clampSelection(state.selected, next),
      };
    }),
  textEdited: (text) => set({ text }),
  textParsed: (model, parseProblem) =>
    set((state) =>
      model ? { model, parseProblem: null, selected: clampSelection(state.selected, model) } : { parseProblem },
    ),
  modeChanged: (mode, text) => set((state) => ({ mode, text: text ?? state.text, parseProblem: null })),
  saved: (model, text) =>
    set({
      model,
      savedFingerprint: flowFingerprint(model),
      text,
      savedText: text,
      parseProblem: null,
      conflict: false,
    }),
  savedBehindEdits: (model, text) =>
    set({ savedFingerprint: flowFingerprint(model), savedText: text, conflict: false }),
  select: (selected) => set((state) => ({ selected, run: { ...state.run, following: false } })),
  panelShown: (panel) => set({ panel }),
  requestsLoaded: (requests) => set((state) => ({ requests: { ...state.requests, ...requests } })),
  runStarted: (steps) =>
    set({
      run: {
        running: true,
        started: Array.from({ length: steps }, () => null),
        results: Array.from({ length: steps }, () => null),
        current: 0,
        last: null,
        following: true,
        stale: false,
      },
      panel: "run",
      selected: steps > 0 ? 0 : null,
    }),
  stepStarted: (start) =>
    set((state) => {
      const started = [...state.run.started];
      started[start.index] = start;
      return {
        run: { ...state.run, started, current: start.index },
        selected: state.run.following ? start.index : state.selected,
      };
    }),
  stepFinished: (result) =>
    set((state) => {
      const results = [...state.run.results];
      results[result.index] = result;
      return { run: { ...state.run, results, current: result.skipped ? null : result.index + 1 } };
    }),
  runFinished: (last) =>
    set((state) => {
      if (!last) return { run: { ...state.run, running: false, current: null } };
      const failedAt = last.steps.findIndex((step) => !step.ok && !step.skipped);
      return {
        run: { ...state.run, running: false, results: last.steps, current: null, last, stale: false },
        selected: state.run.following && failedAt >= 0 ? failedAt : state.selected,
      };
    }),
  conflicted: () => set({ conflict: true }),
  saveStatusChanged: (saveStatus) => set({ saveStatus }),
  asked: (ask) => set({ ask }),
  askClosed: (id) => set((state) => (state.ask?.id === id ? { ask: null } : {})),
  displayed: (message) => set({ message }),
}));

/** Whether there are edits that aren't saved, in whichever mode is showing. */
export const selectFlowDirty = (state: FlowState): boolean => {
  if (!state.model) return false;
  if (state.mode === "yaml") return state.text !== state.savedText;
  return flowFingerprint(state.model) !== state.savedFingerprint;
};
