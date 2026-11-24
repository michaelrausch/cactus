import { create } from "zustand";
import { persist } from "zustand/middleware";
import { DEFAULT_THEME, resolveTheme, type ThemeId } from "@/theme/themes";

interface SettingsState {
  themeId: ThemeId;
  setTheme: (themeId: ThemeId) => void;
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
      setTheme: (themeId) => set({ themeId }),
    }),
    {
      name: "cactus.settings",
      version: 1,
      partialize: ({ themeId }) => ({ themeId }),
      // A theme saved by another version may not exist any more.
      merge: (persisted, current) => ({
        ...current,
        themeId: resolveTheme((persisted as Partial<SettingsState> | undefined)?.themeId).id,
      }),
    },
  ),
);
