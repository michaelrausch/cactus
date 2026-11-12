import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { AlertsBanner } from "@/features/critters/AlertsBanner";
import { NewCritterDialog } from "@/features/critters/NewCritterDialog";
import { ConfirmDialog } from "@/features/dialogs/ConfirmDialog";
import { PromptDialog } from "@/features/dialogs/PromptDialog";
import { EnvDialog } from "@/features/env/EnvDialog";
import { GitDialog } from "@/features/git/GitDialog";
import { ScenarioSwitcher } from "@/features/palette/ScenarioSwitcher";
import { RunDialog } from "@/features/run/RunDialog";
import { SecretsDialogs } from "@/features/secrets/SecretsDialogs";
import { SettingsDialog } from "@/features/settings/SettingsDialog";
import { Sidebar } from "@/features/sidebar/Sidebar";
import { Workspace } from "@/features/workspace/Workspace";
import { useAppLifecycle } from "@/hooks/useAppLifecycle";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";

export default function App() {
  useAppLifecycle();
  useKeyboardShortcuts();

  return (
    <TooltipProvider delayDuration={400}>
      <div className="relative h-screen overflow-hidden bg-background bg-(image:--app-backdrop) text-foreground">
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
        {import.meta.env.SECRETS_ENCRYPTION && <SecretsDialogs />}
        <RunDialog />
        <NewCritterDialog />
        <AlertsBanner />
        <ScenarioSwitcher />
        <Toaster position="bottom-right" />
        {/* Theme effects drawn over everything, e.g. scanlines. */}
        <div aria-hidden className="pointer-events-none absolute inset-0 z-40 bg-(image:--app-overlay)" />
      </div>
    </TooltipProvider>
  );
}
