import type { CSSProperties } from "react";
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

/**
 * A soft glow drawn as a gradient. A `blur()` filter looks the same but is
 * re-rendered on every repaint beneath it, which makes WebKitGTK crawl.
 */
const glow = (color: string): CSSProperties => ({
  background: `radial-gradient(closest-side, ${color}, transparent)`,
});

/** Everything right of the sidebar. */
export function Workspace() {
  return (
    <main className="relative flex h-full flex-col overflow-hidden">
      {/* The glow behind the title bar. */}
      <div
        className="pointer-events-none absolute -top-52 -left-14 h-96 w-[42rem] opacity-[0.2]"
        style={glow("var(--color-brand-2)")}
      />
      <div
        className="pointer-events-none absolute -top-44 right-8 h-80 w-[24rem] opacity-[0.1]"
        style={glow("var(--color-pink-500)")}
      />
      <Header />
      <div className="relative min-h-0 flex-1">
        <EditorArea />
      </div>
    </main>
  );
}
