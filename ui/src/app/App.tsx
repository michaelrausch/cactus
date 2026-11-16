import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { AlertsBanner } from "@/features/critters/AlertsBanner";
import { FlowAskDialog } from "@/features/critters/flows/FlowAskDialog";
import { NewCritterDialog } from "@/features/critters/NewCritterDialog";
import { ConfirmDialog } from "@/features/dialogs/ConfirmDialog";
import { PromptDialog } from "@/features/dialogs/PromptDialog";
import { NewRequestDialog } from "@/features/sidebar/NewRequestDialog";
import { EnvDialog } from "@/features/env/EnvDialog";
import { GitDialog } from "@/features/git/GitDialog";
import { OnboardingDialog } from "@/features/onboarding/OnboardingDialog";
import { ScenarioSwitcher } from "@/features/palette/ScenarioSwitcher";
import { RunDialog } from "@/features/run/RunDialog";
import { SettingsDialog } from "@/features/settings/SettingsDialog";
import { Sidebar } from "@/features/sidebar/Sidebar";
import { LockScreen } from "@/features/vault/LockScreen";
import { Workspace } from "@/features/workspace/Workspace";
import { useAppLifecycle } from "@/hooks/useAppLifecycle";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { useVaultStore } from "@/stores/vaultStore";

export default function App() {
  useAppLifecycle();
  useKeyboardShortcuts();
  const locked = useVaultStore((s) => s.status === "locked");

  return (
    <TooltipProvider delayDuration={400}>
      <div className="relative h-screen overflow-hidden bg-background bg-(image:--app-backdrop) text-foreground">
        {locked ? <LockScreen /> : <Unlocked />}
        <Toaster position="bottom-right" />
        {/* Theme effects drawn over everything, e.g. scanlines. */}
        <div aria-hidden className="pointer-events-none absolute inset-0 z-40 bg-(image:--app-overlay)" />
      </div>
    </TooltipProvider>
  );
}

/** Everything but the lock screen. */
function Unlocked() {
  return (
    <>
      <ResizablePanelGroup orientation="horizontal">
        <ResizablePanel defaultSize="264px" minSize="200px" maxSize="420px">
          <Sidebar />
        </ResizablePanel>
        <ResizableHandle />
        <ResizablePanel>
          <Workspace />
        </ResizablePanel>
      </ResizablePanelGroup>

      <PromptDialog />
      <ConfirmDialog />
      <EnvDialog />
      <GitDialog />
      <SettingsDialog />
      <RunDialog />
      <NewCritterDialog />
      <NewRequestDialog />
      <FlowAskDialog />
      <AlertsBanner />
      <ScenarioSwitcher />
      <OnboardingDialog />
    </>
  );
}
