import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { Toaster } from "@/components/ui/sonner";
import { ConfirmDialog } from "@/features/dialogs/ConfirmDialog";
import { PromptDialog } from "@/features/dialogs/PromptDialog";
import { EnvDialog } from "@/features/env/EnvDialog";
import { ScenarioSwitcher } from "@/features/palette/ScenarioSwitcher";
import { RunDialog } from "@/features/run/RunDialog";
import { SettingsDialog } from "@/features/settings/SettingsDialog";
import { Sidebar } from "@/features/sidebar/Sidebar";
import { Workspace } from "@/features/workspace/Workspace";
import { useAppLifecycle } from "@/hooks/useAppLifecycle";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";

export default function App() {
  useAppLifecycle();
  useKeyboardShortcuts();

  return (
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
      <SettingsDialog />
      <RunDialog />
      <ScenarioSwitcher />
      <Toaster position="bottom-right" />
      {/* Theme effects drawn over everything, e.g. scanlines. */}
      <div aria-hidden className="pointer-events-none absolute inset-0 z-40 bg-(image:--app-overlay)" />
    </div>
  );
}
