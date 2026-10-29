import { create } from "zustand";
import type { HistoryEntry, HttpResponse, ScenarioPath } from "@/api/client";

interface ResponseState {
  response: HttpResponse | null;
  /** Why the last send failed before getting a response. */
  error: string | null;
  history: HistoryEntry[];
  /** Set when `response` came from history rather than a fresh send. */
  historyAt: number | null;
  sending: boolean;
  /** Last status seen per scenario this session, for the sidebar. */
  lastStatusByPath: Record<ScenarioPath, number>;

  showHistory: (history: HistoryEntry[]) => void;
  pickHistory: (entry: HistoryEntry) => void;
  sendStarted: () => void;
  sendSucceeded: (path: ScenarioPath, response: HttpResponse) => void;
  sendFailed: (error: string) => void;
  sendFinished: () => void;
  statusSeen: (path: ScenarioPath, status: number) => void;
  setHistory: (history: HistoryEntry[]) => void;
  clearResponse: () => void;
  reset: () => void;
}

const empty = { response: null, error: null, history: [], historyAt: null };

export const useResponseStore = create<ResponseState>()((set) => ({
  ...empty,
  sending: false,
  lastStatusByPath: {},

  /** Show the newest history entry, as if it had just been sent. */
  showHistory: (history) =>
    set({ history, response: history[0]?.response ?? null, historyAt: history[0]?.at ?? null, error: null }),
  pickHistory: (entry) => set({ response: entry.response, historyAt: entry.at, error: null }),
  sendStarted: () => set({ sending: true, error: null }),
  sendSucceeded: (path, response) =>
    set((s) => ({
      response,
      historyAt: null,
      lastStatusByPath: { ...s.lastStatusByPath, [path]: response.status },
    })),
  sendFailed: (error) => set({ response: null, error }),
  sendFinished: () => set({ sending: false }),
  statusSeen: (path, status) => set((s) => ({ lastStatusByPath: { ...s.lastStatusByPath, [path]: status } })),
  setHistory: (history) => set({ history }),
  clearResponse: () => set(empty),
  reset: () => set({ ...empty, lastStatusByPath: {} }),
}));
