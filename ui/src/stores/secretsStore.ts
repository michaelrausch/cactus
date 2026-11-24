import { create } from "zustand";
import type { SecretsInfo, SecretsStatus } from "@/api/client";

/** Which secrets dialog is open. */
export type SecretsDialog = "enable" | "unlock" | "changePassword" | "disable" | "reset";

interface SecretsState {
  /** `null` until loaded for the open project. */
  info: SecretsInfo | null;
  dialog: SecretsDialog | null;

  reset: () => void;
  loaded: (info: SecretsInfo) => void;
  statusChanged: (status: SecretsStatus) => void;
  setDialog: (dialog: SecretsDialog | null) => void;
}

export const useSecretsStore = create<SecretsState>()((set) => ({
  info: null,
  dialog: null,

  reset: () => set({ info: null, dialog: null }),
  loaded: (info) => set({ info }),
  statusChanged: (status) => set((state) => (state.info ? { info: { ...state.info, status } } : {})),
  setDialog: (dialog) => set({ dialog }),
}));
