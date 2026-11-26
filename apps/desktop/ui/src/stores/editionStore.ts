import { create } from "zustand";

interface EditionState {
  /** Who the Pro edition is licensed to; null until a key is entered, and always in the free edition. */
  licensee: string | null;
  /** Whether the backend has said yet. */
  loaded: boolean;
  editionLoaded: (licensee: string | null) => void;
}

export const useEditionStore = create<EditionState>()((set) => ({
  licensee: null,
  loaded: false,
  editionLoaded: (licensee) => set({ licensee, loaded: true }),
}));
