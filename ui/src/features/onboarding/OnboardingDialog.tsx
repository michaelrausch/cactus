import { useState, type ComponentType } from "react";
import { FolderOpen } from "lucide-react";
import { finishOnboarding, openProject } from "@/actions/project";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { SPECIES, SPECIES_IDS } from "@/components/critters/species";
import { Logo } from "@/components/Logo";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { ThemeCard } from "@/features/settings/ThemeTab";
import { cn } from "@/lib/utils";
import { useProjectStore } from "@/stores/projectStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { useUiStore } from "@/stores/uiStore";
import { THEMES } from "@/theme/themes";

const STEPS = ["welcome", "theme", "project"] as const;
type Step = (typeof STEPS)[number];

function WelcomeStep() {
  return (
    <div className="flex flex-col items-center gap-4 text-center">
      <Logo className="size-20 drop-shadow-[0_0_18px_color-mix(in_oklch,var(--color-brand-2)_40%,transparent)]" />
      <DialogTitle className="text-brand text-3xl font-semibold tracking-tight">cactus</DialogTitle>
      <div className="mt-6 flex gap-2">
        {SPECIES_IDS.map((species) => (
          <CritterSprite key={species} species={species} size={48} label={SPECIES[species].name} grounded />
        ))}
      </div>
    </div>
  );
}

function ThemeStep() {
  const themeId = useSettingsStore((s) => s.themeId);
  const setTheme = useSettingsStore((s) => s.setTheme);
  return (
    <div className="flex min-h-0 flex-col gap-4 py-4">
      <DialogTitle className="text-2xl font-semibold tracking-tight">Theme</DialogTitle>
      <div role="radiogroup" aria-label="Theme" className="grid grid-cols-2 gap-2 sm:grid-cols-3">
        {THEMES.map((theme) => (
          <ThemeCard key={theme.id} theme={theme} selected={theme.id === themeId} onSelect={() => setTheme(theme.id)} />
        ))}
      </div>
    </div>
  );
}

async function openFirstProject(): Promise<void> {
  await openProject();
  // Picking was cancelled: stay on this step.
  if (useProjectStore.getState().project) finishOnboarding();
}

function ProjectStep() {
  return (
    <div className="flex flex-col items-center gap-4 text-center">
      <DialogTitle className="text-2xl font-semibold tracking-tight">Create/open project</DialogTitle>
      <button
        type="button"
        onClick={() => void openFirstProject()}
        className="flex items-center gap-2 rounded-full bg-brand-2 px-4 py-2 text-sm font-medium text-brand-foreground shadow-lg shadow-brand-2/20 focus-ring hover:brightness-110"
      >
        <FolderOpen className="size-4" /> Open folder
      </button>
    </div>
  );
}

const STEP_VIEWS: Record<Step, ComponentType> = {
  welcome: WelcomeStep,
  theme: ThemeStep,
  project: ProjectStep,
};

function StepDots({ index }: { index: number }) {
  return (
    <div className="mr-auto flex items-center gap-1.5" aria-label={`Step ${index + 1} of ${STEPS.length}`}>
      {STEPS.map((step, i) => (
        <span
          key={step}
          className={cn("size-1.5 rounded-full bg-muted-foreground/30 transition", i === index && "bg-brand-2")}
        />
      ))}
    </div>
  );
}

/** Mounted per opening, so it starts on the first step. */
function Onboarding() {
  const [index, setIndex] = useState(0);
  const step = STEPS[index] ?? "welcome";
  const View = STEP_VIEWS[step];
  const last = index === STEPS.length - 1;
  return (
    <div className="flex h-full flex-col">
      {/* The window's title bar is hidden behind this layer, so it needs its own drag strip. */}
      <div data-tauri-drag-region className="h-10 shrink-0" />
      {/* `my-auto` rather than centring the grid, so a step taller than the window still scrolls to its top. */}
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-8">
        <div className="mx-auto my-auto w-full max-w-3xl">
          <View />
        </div>
      </div>
      <div className="mx-auto flex w-full max-w-3xl shrink-0 items-center gap-2 px-8 pt-4 pb-8">
        <StepDots index={index} />
        <Button variant="ghost" onClick={finishOnboarding}>
          Skip
        </Button>
        {index > 0 && (
          <Button variant="ghost" onClick={() => setIndex(index - 1)}>
            Back
          </Button>
        )}
        {!last && (
          <Button autoFocus onClick={() => setIndex(index + 1)}>
            {index === 0 ? "Get started" : "Next"}
          </Button>
        )}
      </div>
    </div>
  );
}

/** Shown once, on first launch. Dismissing it counts as seen. */
export function OnboardingDialog() {
  const open = useUiStore((s) => s.onboardingOpen);
  return (
    <Dialog open={open} onOpenChange={(next) => !next && finishOnboarding()}>
      <DialogContent
        showCloseButton={false}
        aria-describedby={undefined}
        className="inset-0 top-0 left-0 flex h-full w-full max-w-none translate-x-0 translate-y-0 flex-col gap-0 rounded-none bg-background bg-(image:--app-backdrop) p-0 ring-0 sm:max-w-none"
      >
        {open && <Onboarding />}
      </DialogContent>
    </Dialog>
  );
}
