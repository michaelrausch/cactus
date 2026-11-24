import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { Toaster } from "@/components/ui/sonner";
import { ConfirmDialog } from "@/features/dialogs/ConfirmDialog";
import { PromptDialog } from "@/features/dialogs/PromptDialog";
import { EnvDialog } from "@/features/env/EnvDialog";
import { ScenarioSwitcher } from "@/features/palette/ScenarioSwitcher";
import { RunDialog } from "@/features/run/RunDialog";
import { Sidebar } from "@/features/sidebar/Sidebar";
import { Workspace } from "@/features/workspace/Workspace";
import { useAppLifecycle } from "@/hooks/useAppLifecycle";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";

export default function App() {
  useAppLifecycle();
  useKeyboardShortcuts();

  return (
    <div className="dark h-screen overflow-hidden bg-background text-foreground">
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
      <RunDialog />
      <ScenarioSwitcher />
      <Toaster position="bottom-right" />
    </div>
  );
}
