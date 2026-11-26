import { create } from "zustand";
import { persist } from "zustand/middleware";
import { storageKey } from "@/lib/storage";
import { DEFAULT_THEME, resolveTheme, type ThemeId } from "@/theme/themes";

/** Who commits are by. Blank parts fall back to git config. */
export interface CommitIdentity {
  name: string;
  email: string;
}

interface SettingsState {
  themeId: ThemeId;
  commitIdentity: CommitIdentity;
  /** Whether idle critters bob and blink. */
  animateCritters: boolean;
  /** Whether the first-launch onboarding has been seen. */
  onboarded: boolean;
  setTheme: (themeId: ThemeId) => void;
  setCommitIdentity: (identity: Partial<CommitIdentity>) => void;
  setAnimateCritters: (animate: boolean) => void;
  setOnboarded: (onboarded: boolean) => void;
}

/**
 * App-wide preferences, kept in localStorage. It is read synchronously, so the
 * saved theme is in place before the first paint.
 */
/** The current theme, resolved from the saved id. */
export const useTheme = () => resolveTheme(useSettingsStore((s) => s.themeId));

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      themeId: DEFAULT_THEME,
      commitIdentity: { name: "", email: "" },
      animateCritters: true,
      onboarded: false,
      setTheme: (themeId) => set({ themeId }),
      setCommitIdentity: (identity) => set((s) => ({ commitIdentity: { ...s.commitIdentity, ...identity } })),
      setAnimateCritters: (animateCritters) => set({ animateCritters }),
      setOnboarded: (onboarded) => set({ onboarded }),
    }),
    {
      name: storageKey("settings"),
      version: 1,
      partialize: ({ themeId, commitIdentity, animateCritters, onboarded }) => ({
        themeId,
        commitIdentity,
        animateCritters,
        onboarded,
      }),
      merge: (persisted, current) => {
        const saved = persisted as Partial<SettingsState> | undefined;
        return {
          ...current,
          // A theme saved by another version may not exist any more.
          themeId: resolveTheme(saved?.themeId).id,
          commitIdentity: { ...current.commitIdentity, ...saved?.commitIdentity },
          animateCritters: saved?.animateCritters ?? current.animateCritters,
          onboarded: saved?.onboarded ?? current.onboarded,
        };
      },
    },
  ),
);
