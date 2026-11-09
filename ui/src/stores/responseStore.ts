import { create } from "zustand";
import type { HistoryEntry, HttpResponse, ProjectPath } from "@/api/client";

interface ResponseState {
  response: HttpResponse | null;
  /** Why the last send failed before getting a response. */
  error: string | null;
  history: HistoryEntry[];
  /** Set when `response` came from history rather than a fresh send. */
  historyAt: number | null;
  sending: boolean;
  /** Bumped whenever another response is shown, so views of it can start fresh. */
  shownVersion: number;
  /** Last status seen per request this session, for the sidebar. */
  lastStatusByPath: Record<ProjectPath, number>;

  historyLoaded: (history: HistoryEntry[]) => void;
  historyEntryPicked: (entry: HistoryEntry) => void;
  sendStarted: () => void;
  sendSucceeded: (path: ProjectPath, response: HttpResponse) => void;
  sendFailed: (error: string) => void;
  sendFinished: () => void;
  statusSeen: (path: ProjectPath, status: number) => void;
  historyUpdated: (history: HistoryEntry[]) => void;
  cleared: () => void;
  reset: () => void;
}

/** Whether the response pane has anything to show: a response, an error, or a send under way. */
export const hasResponseContent = (s: Pick<ResponseState, "response" | "error" | "sending">): boolean =>
  s.response !== null || s.error !== null || s.sending;

const empty = { response: null, error: null, history: [], historyAt: null };

export const useResponseStore = create<ResponseState>()((set) => ({
  ...empty,
  sending: false,
  shownVersion: 0,
  lastStatusByPath: {},

  /** Show the newest history entry, as if it had just been sent. */
  historyLoaded: (history) =>
    set((s) => ({
      history,
      response: history[0]?.response ?? null,
      historyAt: history[0]?.at ?? null,
      error: null,
      shownVersion: s.shownVersion + 1,
    })),
  historyEntryPicked: (entry) =>
    set((s) => ({ response: entry.response, historyAt: entry.at, error: null, shownVersion: s.shownVersion + 1 })),
  sendStarted: () => set({ sending: true, error: null }),
  sendSucceeded: (path, response) =>
    set((s) => ({
      response,
      historyAt: null,
      shownVersion: s.shownVersion + 1,
      lastStatusByPath: { ...s.lastStatusByPath, [path]: response.status },
    })),
  sendFailed: (error) => set({ response: null, error }),
  sendFinished: () => set({ sending: false }),
  statusSeen: (path, status) => set((s) => ({ lastStatusByPath: { ...s.lastStatusByPath, [path]: status } })),
  historyUpdated: (history) => set({ history }),
  cleared: () => set(empty),
  reset: () => set({ ...empty, lastStatusByPath: {} }),
}));
