import { RotateCcw } from "lucide-react";
import { resetAchievements } from "@/actions/achievements";
import { ACHIEVEMENT_ICONS } from "@/components/achievements/icons";
import { Button } from "@/components/ui/button";
import { ACHIEVEMENTS, type Achievement } from "@/domain/achievements";
import { cn } from "@/lib/utils";
import { useAchievementStore } from "@/stores/achievementStore";

const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

function AchievementRow({ achievement }: { achievement: Achievement }) {
  const count = useAchievementStore((s) => s.counts[achievement.stat] ?? 0);
  const unlockedAt = useAchievementStore((s) => s.unlocked[achievement.id]);
  const Icon = ACHIEVEMENT_ICONS[achievement.icon];
  const progress = Math.min(count, achievement.goal);

  return (
    <div className="flex items-center gap-3 px-3 py-2.5">
      <div
        className={cn(
          "flex size-8 shrink-0 items-center justify-center rounded-full [&_svg]:size-4",
          unlockedAt ? "bg-brand-2 text-brand-foreground" : "bg-muted text-muted-foreground/60",
        )}
      >
        <Icon />
      </div>
      <div className={cn("min-w-0 flex-1", !unlockedAt && "opacity-60")}>
        <p className="text-sm font-medium">{achievement.name}</p>
        <p className="text-xs text-muted-foreground">{achievement.description}</p>
      </div>
      {unlockedAt && <span className="text-xs text-muted-foreground">{dateFormat.format(new Date(unlockedAt))}</span>}
      {!unlockedAt && achievement.goal > 1 && <Progress value={progress} goal={achievement.goal} />}
    </div>
  );
}

function Progress({ value, goal }: { value: number; goal: number }) {
  return (
    <div className="flex w-28 flex-col items-end gap-1">
      <span className="font-mono text-xs text-muted-foreground tabular-nums">
        {value.toLocaleString()} / {goal.toLocaleString()}
      </span>
      <div className="h-1 w-full overflow-hidden rounded-full bg-muted">
        <div className="h-full bg-primary/70" style={{ width: `${(value / goal) * 100}%` }} />
      </div>
    </div>
  );
}

export function AchievementsTab() {
  const unlocked = useAchievementStore(
    (s) => ACHIEVEMENTS.filter((achievement) => achievement.id in s.unlocked).length,
  );
  return (
    <section className="flex max-w-lg flex-col gap-2">
      <div className="flex items-baseline justify-between">
        <h3 className="text-xs font-medium tracking-wide text-muted-foreground uppercase">Achievements</h3>
        <span className="font-mono text-xs text-muted-foreground tabular-nums">
          {unlocked} / {ACHIEVEMENTS.length}
        </span>
      </div>
      <div className="flex flex-col divide-y overflow-hidden rounded-xl border bg-card/60">
        {ACHIEVEMENTS.map((achievement) => (
          <AchievementRow key={achievement.id} achievement={achievement} />
        ))}
      </div>
      <div>
        <Button variant="ghost" size="sm" onClick={resetAchievements}>
          <RotateCcw /> Reset
        </Button>
      </div>
    </section>
  );
}
