import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Counts, Stat } from "@/domain/achievements";

interface AchievementState {
  counts: Counts;
  /** Achievement id → when it was unlocked (ISO). */
  unlocked: Record<string, string>;
  /** For stats that count distinct things (themes, projects): the ones seen so far. */
  seen: Partial<Record<Stat, string[]>>;
  bump: (stat: Stat, by: number) => void;
  /** Count `key` towards `stat` the first time it is seen. */
  see: (stat: Stat, key: string) => void;
  unlock: (ids: string[], at: string) => void;
  reset: () => void;
}

/** Achievement progress, app-wide in localStorage rather than per project. */
export const useAchievementStore = create<AchievementState>()(
  persist(
    (set) => ({
      counts: {},
      unlocked: {},
      seen: {},
      bump: (stat, by) => set((s) => ({ counts: { ...s.counts, [stat]: (s.counts[stat] ?? 0) + by } })),
      see: (stat, key) =>
        set((s) => {
          const keys = s.seen[stat] ?? [];
          if (keys.includes(key)) return s;
          const next = [...keys, key];
          return { seen: { ...s.seen, [stat]: next }, counts: { ...s.counts, [stat]: next.length } };
        }),
      unlock: (ids, at) =>
        set((s) => ({ unlocked: { ...s.unlocked, ...Object.fromEntries(ids.map((id) => [id, at])) } })),
      reset: () => set({ counts: {}, unlocked: {}, seen: {} }),
    }),
    {
      name: "cactus.achievements",
      version: 1,
      partialize: ({ counts, unlocked, seen }) => ({ counts, unlocked, seen }),
    },
  ),
);
