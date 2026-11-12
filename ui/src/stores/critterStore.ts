import { create } from "zustand";
import type { ContextEntry, CritterSummary } from "@/api/client";

/** A critter's captured values, by name. */
export type CritterContext = Record<string, ContextEntry>;

interface CritterState {
  /** Every critter in the project, sorted by id. */
  critters: CritterSummary[];
  /** Whether `critters` has been listed for the open project, so "none" can be told from "not yet". */
  loaded: boolean;
  /** The critter requests are sent as, or null for none. */
  activeId: string | null;
  /** The critter last shown in the main pane, or whose flow is. */
  selectedId: string | null;
  /** Each critter's captured values, once loaded. */
  contexts: Record<string, CritterContext>;

  reset: () => void;
  listed: (critters: CritterSummary[], activeId: string | null) => void;
  activeChanged: (id: string | null) => void;
  select: (id: string | null) => void;
  contextLoaded: (id: string, context: CritterContext) => void;
}

export const useCritterStore = create<CritterState>()((set) => ({
  critters: [],
  loaded: false,
  activeId: null,
  selectedId: null,
  contexts: {},

  reset: () => set({ critters: [], loaded: false, activeId: null, selectedId: null, contexts: {} }),
  listed: (critters, activeId) =>
    set((state) => ({
      critters,
      loaded: true,
      activeId,
      // Keep the selection while it still exists; otherwise fall back to the first.
      selectedId: critters.some((c) => c.id === state.selectedId) ? state.selectedId : (critters[0]?.id ?? null),
      // Forget the contexts of critters that are gone.
      contexts: Object.fromEntries(
        Object.entries(state.contexts).filter(([id]) => critters.some((critter) => critter.id === id)),
      ),
    })),
  activeChanged: (activeId) => set({ activeId }),
  select: (selectedId) => set({ selectedId }),
  contextLoaded: (id, context) => set((state) => ({ contexts: { ...state.contexts, [id]: context } })),
}));

/** The summary of the critter requests are sent as. */
export const selectActiveCritter = (state: CritterState): CritterSummary | null =>
  state.critters.find((critter) => critter.id === state.activeId) ?? null;

/** The summary of the critter last shown in the main pane. */
export const selectSelectedCritter = (state: CritterState): CritterSummary | null =>
  state.critters.find((critter) => critter.id === state.selectedId) ?? null;
