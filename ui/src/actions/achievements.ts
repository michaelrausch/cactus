import { showAchievementToast } from "@/components/achievements/AchievementToast";
import { newlyUnlocked, type Stat } from "@/domain/achievements";
import { useAchievementStore } from "@/stores/achievementStore";
import { useUiStore } from "@/stores/uiStore";

function checkUnlocks(): void {
  const { counts, unlocked, unlock } = useAchievementStore.getState();
  const reached = newlyUnlocked(counts, unlocked);
  if (reached.length === 0) return;
  unlock(
    reached.map((achievement) => achievement.id),
    new Date().toISOString(),
  );
  // One toast at most; any others reached at the same moment are still unlocked and listed in Settings.
  const last = reached.at(-1);
  if (last) showAchievementToast(last);
}

/** Count something done in the window towards achievements. */
export function track(stat: Stat, by = 1): void {
  useAchievementStore.getState().bump(stat, by);
  checkUnlocks();
}

/** Ask, then clear every count and unlocked achievement. */
export function resetAchievements(): void {
  useUiStore.getState().askConfirm({
    title: "Reset achievements?",
    description: "Every achievement and count starts again from zero.",
    action: "Reset",
    destructive: true,
    onConfirm: () => useAchievementStore.getState().reset(),
  });
}

/** Count `key` towards `stat` once, however often it comes up (a theme, a project). */
export function trackDistinct(stat: Stat, key: string): void {
  useAchievementStore.getState().see(stat, key);
  checkUnlocks();
}
