import { create } from "zustand";

export interface PromptRequest {
  title: string;
  description?: string;
  initial: string;
  placeholder?: string;
  onSubmit: (value: string) => Promise<void> | void;
}

export interface ConfirmRequest {
  title: string;
  description: string;
  action: string;
  onConfirm: () => Promise<void> | void;
  /** Whether the main action destroys something; true unless set. */
  destructive?: boolean;
  /** A second choice shown left of the main action, e.g. "Discard". */
  alternative?: { action: string; onConfirm: () => Promise<void> | void };
}

interface UiState {
  prompt: PromptRequest | null;
  confirm: ConfirmRequest | null;
  envDialogOpen: boolean;
  switcherOpen: boolean;
  responseVisible: boolean;

  askPrompt: (prompt: PromptRequest) => void;
  askConfirm: (confirm: ConfirmRequest) => void;
  closePrompt: () => void;
  closeConfirm: () => void;
  setEnvDialogOpen: (open: boolean) => void;
  setSwitcherOpen: (open: boolean) => void;
  toggleSwitcher: () => void;
  toggleResponse: () => void;
}

export const useUiStore = create<UiState>()((set) => ({
  prompt: null,
  confirm: null,
  envDialogOpen: false,
  switcherOpen: false,
  responseVisible: true,

  askPrompt: (prompt) => set({ prompt }),
  askConfirm: (confirm) => set({ confirm }),
  closePrompt: () => set({ prompt: null }),
  closeConfirm: () => set({ confirm: null }),
  setEnvDialogOpen: (envDialogOpen) => set({ envDialogOpen }),
  setSwitcherOpen: (switcherOpen) => set({ switcherOpen }),
  toggleSwitcher: () => set((s) => ({ switcherOpen: !s.switcherOpen })),
  toggleResponse: () => set((s) => ({ responseVisible: !s.responseVisible })),
}));
