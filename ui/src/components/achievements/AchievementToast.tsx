import confetti from "canvas-confetti";
import { toast } from "sonner";
import type { Achievement } from "@/domain/achievements";
import { ACHIEVEMENT_ICONS } from "./icons";

/** Off for now; flip to bring the confetti burst back. */
const CONFETTI = false;

function AchievementToast({ achievement }: { achievement: Achievement }) {
  const Icon = ACHIEVEMENT_ICONS[achievement.icon];
  return (
    <div className="flex w-full justify-center">
      <div className="achievement-pill flex items-center rounded-full border bg-popover p-2 text-popover-foreground shadow-lg">
        <div className="achievement-medal relative flex size-12 shrink-0 items-center justify-center rounded-full bg-brand text-brand-foreground [&_svg]:size-6">
          <span aria-hidden className="achievement-ripple" />
          <Icon strokeWidth={2.25} />
        </div>
        <div className="achievement-reveal overflow-hidden">
          <div className="achievement-text flex w-64 flex-col gap-0.5 pr-4 pl-3">
            <p className="w-fit text-brand text-2xs font-medium">Achievement unlocked</p>
            <p className="truncate text-base leading-tight font-semibold tracking-tight">{achievement.name}</p>
            <p className="truncate text-xs leading-tight text-muted-foreground">{achievement.description}</p>
          </div>
        </div>
      </div>
    </div>
  );
}

export function showAchievementToast(achievement: Achievement): void {
  toast.custom(() => <AchievementToast achievement={achievement} />, { duration: 3500, position: "top-center" });
  if (!CONFETTI) return;
  // From the toast, once the pill has widened.
  window.setTimeout(
    () =>
      void confetti({
        particleCount: 120,
        spread: 80,
        startVelocity: 40,
        angle: 270,
        origin: { x: 0.5, y: 0.08 },
        disableForReducedMotion: true,
      }),
    700,
  );
}
