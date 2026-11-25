import { create } from "zustand";
import type { VaultStatus } from "@/api/bindings";

interface VaultState {
  /** The open project's status, or null before it's known. While `locked`, the lock screen covers the app. */
  status: VaultStatus | null;
  /** Unlocking takes an authenticator code too. Only known once unlocked. */
  totp: boolean;
  statusLoaded: (status: VaultStatus | null, totp: boolean) => void;
}

export const useVaultStore = create<VaultState>()((set) => ({
  status: null,
  totp: false,
  statusLoaded: (status, totp) => set({ status, totp }),
}));
