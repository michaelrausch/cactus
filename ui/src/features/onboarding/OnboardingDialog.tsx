import { useState, type ComponentType } from "react";
import { PRO } from "@edition/flag";
import { Bot, FolderOpen, GitBranch, Import, Users, Workflow, type LucideIcon } from "lucide-react";
import { finishOnboarding, importCollection, openProject } from "@/actions/project";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { SPECIES, SPECIES_IDS } from "@/components/critters/species";
import { Logo } from "@/components/Logo";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { ThemePicker } from "@/features/settings/ThemeTab";
import { cn } from "@/lib/utils";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

const STEPS = ["welcome", "theme", "project"] as const;
type Step = (typeof STEPS)[number];

const FEATURES: { icon: LucideIcon; name: string; detail: string }[] = [
  { icon: Users, name: "Critters", detail: "Test as different users, each kept in its own sandbox" },
  { icon: Workflow, name: "Flows", detail: "Play out real workflows across your APIs and tools" },
  { icon: Bot, name: "MCP", detail: "Let AI agents call your APIs without handing over your keys" },
  { icon: GitBranch, name: "Git", detail: "Your work lives in git, so there's nothing to host" },
];

function WelcomeStep() {
  return (
    <div className="flex flex-col items-center gap-4 text-center">
      <Logo className="size-20 drop-shadow-[0_0_18px_color-mix(in_oklch,var(--color-brand-2)_40%,transparent)]" />
      <DialogTitle className="text-brand text-3xl font-semibold tracking-tight">
        {PRO ? "cactus pro" : "cactus"}
      </DialogTitle>
      <p className="max-w-md text-muted-foreground">An API client for building and testing APIs.</p>
      <ul className="mt-4 grid w-full max-w-xl grid-cols-1 gap-3 text-left sm:grid-cols-2">
        {FEATURES.map(({ icon: Icon, name, detail }) => (
          <li key={name} className="flex items-start gap-3.5 rounded-xl border bg-card/60 p-4 shadow-sm">
            <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-brand-2/15 text-brand-2 ring-1 ring-brand-2/25">
              <Icon className="size-4" />
            </span>
            <span className="flex flex-col gap-1">
              <span className="text-sm font-semibold tracking-tight">{name}</span>
              <span className="text-xs leading-relaxed text-muted-foreground">{detail}</span>
            </span>
          </li>
        ))}
      </ul>
      <div className="mt-6 flex gap-2">
        {SPECIES_IDS.map((species) => (
          <CritterSprite key={species} species={species} size={48} label={SPECIES[species].name} grounded />
        ))}
      </div>
    </div>
  );
}

function ThemeStep() {
  return (
    <div className="flex min-h-0 flex-col gap-4 py-4">
      <div className="flex flex-col gap-1">
        <DialogTitle className="text-2xl font-semibold tracking-tight">Theme</DialogTitle>
        <p className="text-sm text-muted-foreground">Pick a look you like. You can change it anytime in Settings.</p>
      </div>
      <ThemePicker columns="grid-cols-2 sm:grid-cols-3" />
    </div>
  );
}

/** Open a folder, then (if one was picked) leave onboarding and optionally import into it. */
async function openFirstProject(thenImport = false): Promise<void> {
  await openProject();
  // Picking was cancelled: stay on this step.
  if (!useProjectStore.getState().project) return;
  finishOnboarding();
  if (thenImport) await importCollection();
}

function ProjectChoice({
  icon: Icon,
  label,
  detail,
  primary = false,
  onSelect,
}: {
  icon: LucideIcon;
  label: string;
  detail: string;
  primary?: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className="group flex w-60 flex-col items-center gap-3 rounded-xl border bg-card/60 px-5 pt-6 pb-5 shadow-sm focus-ring transition hover:-translate-y-0.5 hover:border-brand-2/60 hover:bg-accent/50"
    >
      <span
        className={cn(
          "flex size-12 items-center justify-center rounded-full transition group-hover:scale-105",
          primary ? "bg-brand-2 text-brand-foreground shadow-lg shadow-brand-2/25" : "bg-muted text-foreground",
        )}
      >
        <Icon className="size-5" />
      </span>
      <span className="flex flex-col gap-1">
        <span className="text-sm font-semibold tracking-tight">{label}</span>
        <span className="text-xs leading-relaxed text-muted-foreground">{detail}</span>
      </span>
    </button>
  );
}

function ProjectStep() {
  return (
    <div className="flex flex-col items-center gap-8 text-center">
      <div className="flex flex-col gap-1">
        <DialogTitle className="text-2xl font-semibold tracking-tight">Open a project</DialogTitle>
        <p className="text-sm text-muted-foreground">Choose a folder to get started</p>
      </div>
      <div className="flex flex-wrap justify-center gap-3">
        <ProjectChoice
          icon={FolderOpen}
          label="Open folder"
          detail="New or existing project"
          primary
          onSelect={() => void openFirstProject()}
        />
        <ProjectChoice
          icon={Import}
          label="Import collection"
          detail="From Postman or OpenAPI"
          onSelect={() => void openFirstProject(true)}
        />
      </div>
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
    <div className="flex items-center gap-1.5" aria-label={`Step ${index + 1} of ${STEPS.length}`}>
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
      <div className="mx-auto grid w-full max-w-3xl shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-2 px-8 pt-4 pb-8">
        <div className="flex">
          {index > 0 && (
            <Button variant="ghost" onClick={() => setIndex(index - 1)}>
              Back
            </Button>
          )}
        </div>
        <StepDots index={index} />
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={finishOnboarding}>
            Skip
          </Button>
          {!last && (
            <Button autoFocus onClick={() => setIndex(index + 1)}>
              {index === 0 ? "Get started" : "Next"}
            </Button>
          )}
        </div>
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
