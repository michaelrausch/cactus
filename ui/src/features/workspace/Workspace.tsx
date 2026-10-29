import { editScenario } from "@/actions/scenario";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { RequestEditor } from "@/features/editor/RequestEditor";
import { ResponsePanel } from "@/features/response/ResponsePanel";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import { Header } from "./Header";
import { WelcomeScreen } from "./WelcomeScreen";

function ResponsePane() {
  return (
    <ResizablePanel defaultSize="46%" minSize="280px">
      <div className="h-full pr-3 pb-3">
        <div className="h-full overflow-hidden rounded-xl border bg-card/70 shadow-lg shadow-black/20">
          <ResponsePanel />
        </div>
      </div>
    </ResizablePanel>
  );
}

function EditorArea() {
  const path = useEditorStore((s) => s.path);
  const scenario = useEditorStore((s) => s.scenario);
  const editorVersion = useEditorStore((s) => s.editorVersion);
  const responseVisible = useUiStore((s) => s.responseVisible);

  if (!path || !scenario) return <WelcomeScreen />;
  return (
    <ResizablePanelGroup orientation="horizontal">
      <ResizablePanel minSize="320px">
        <RequestEditor key={`${path}:${editorVersion}`} scenario={scenario} onChange={editScenario} />
      </ResizablePanel>
      {responseVisible && (
        <>
          <ResizableHandle className="bg-transparent" />
          <ResponsePane />
        </>
      )}
    </ResizablePanelGroup>
  );
}

/** Everything right of the sidebar. */
export function Workspace() {
  return (
    <main className="relative flex h-full flex-col overflow-hidden">
      {/* The glow behind the title bar. */}
      <div className="pointer-events-none absolute -top-28 left-10 h-48 w-[34rem] rounded-full bg-brand opacity-[0.09] blur-3xl" />
      <div className="pointer-events-none absolute -top-20 right-24 h-32 w-64 rounded-full bg-pink-500 opacity-[0.05] blur-3xl" />
      <Header />
      <div className="relative min-h-0 flex-1">
        <EditorArea />
      </div>
    </main>
  );
}
