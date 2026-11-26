import { create } from "zustand";

export interface PromptRequest {
  title: string;
  description?: string;
  initial: string;
  placeholder?: string;
  /** Resolves to whether the prompt is done; on `false` (e.g. the work failed) it stays open. */
  onSubmit: (value: string) => Promise<boolean> | boolean;
}

export interface ConfirmChoice {
  action: string;
  onConfirm: () => Promise<void> | void;
  /** Whether choosing it destroys something (unsaved edits, a file), which is shown in red. */
  destructive?: boolean;
}

/** The main choice; a destructive one never takes Enter, so focus starts on Cancel. */
export interface ConfirmRequest extends ConfirmChoice {
  title: string;
  description: string;
  /** A second choice shown left of the main one, e.g. "Discard changes". */
  alternative?: ConfirmChoice;
  /** Called when the dialog is cancelled or dismissed, so a caller waiting on the answer isn't left hanging. */
  onCancel?: () => void;
}

/** What the main pane shows: the open request, the selected critter, or the open flow. */
export type MainView = "editor" | "critter" | "flow";

/** The tab shown for the selected critter. */
export type CritterTab = "context" | "settings";

interface UiState {
  view: MainView;
  critterTab: CritterTab;
  newCritterOpen: boolean;
  /** The folder the New request dialog creates in, while it is open. */
  newRequestDir: string | null;
  prompt: PromptRequest | null;
  confirm: ConfirmRequest | null;
  envDialogOpen: boolean;
  gitDialogOpen: boolean;
  settingsOpen: boolean;
  onboardingOpen: boolean;
  switcherOpen: boolean;
  responseVisible: boolean;

  setView: (view: MainView) => void;
  setCritterTab: (tab: CritterTab) => void;
  setNewCritterOpen: (open: boolean) => void;
  setNewRequestDir: (dir: string | null) => void;
  askPrompt: (prompt: PromptRequest) => void;
  askConfirm: (confirm: ConfirmRequest) => void;
  closePrompt: () => void;
  closeConfirm: () => void;
  setEnvDialogOpen: (open: boolean) => void;
  setGitDialogOpen: (open: boolean) => void;
  setSettingsOpen: (open: boolean) => void;
  setOnboardingOpen: (open: boolean) => void;
  setSwitcherOpen: (open: boolean) => void;
  toggleSwitcher: () => void;
  toggleResponse: () => void;
  setResponseVisible: (visible: boolean) => void;
}

export const useUiStore = create<UiState>()((set) => ({
  view: "editor",
  critterTab: "context",
  newCritterOpen: false,
  newRequestDir: null,
  prompt: null,
  confirm: null,
  envDialogOpen: false,
  gitDialogOpen: false,
  settingsOpen: false,
  onboardingOpen: false,
  switcherOpen: false,
  responseVisible: true,

  setView: (view) => set({ view }),
  setCritterTab: (critterTab) => set({ critterTab }),
  setNewCritterOpen: (newCritterOpen) => set({ newCritterOpen }),
  setNewRequestDir: (newRequestDir) => set({ newRequestDir }),
  askPrompt: (prompt) => set({ prompt }),
  askConfirm: (confirm) => set({ confirm }),
  closePrompt: () => set({ prompt: null }),
  closeConfirm: () => set({ confirm: null }),
  setEnvDialogOpen: (envDialogOpen) => set({ envDialogOpen }),
  setGitDialogOpen: (gitDialogOpen) => set({ gitDialogOpen }),
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
  setOnboardingOpen: (onboardingOpen) => set({ onboardingOpen }),
  setSwitcherOpen: (switcherOpen) => set({ switcherOpen }),
  toggleSwitcher: () => set((s) => ({ switcherOpen: !s.switcherOpen })),
  toggleResponse: () => set((s) => ({ responseVisible: !s.responseVisible })),
  setResponseVisible: (responseVisible) => set({ responseVisible }),
}));
