import { memo, useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import { usePanelRef } from "react-resizable-panels";
import { discardChanges, editScenario } from "@/actions/scenario";
import type { ProjectPath } from "@/api/client";
import { reloadFromDisk, saveScenario } from "@/actions/saving";
import { Button } from "@/components/ui/button";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { CritterPage } from "@/features/critters/CritterPage";
import { FlowEditor } from "@/features/critters/flows/FlowEditor";
import { RequestEditor } from "@/features/editor/RequestEditor";
import { ResponsePanel } from "@/features/response/ResponsePanel";
import { selectSelectedCritter, useCritterStore } from "@/stores/critterStore";
import { useEditorStore, type Conflict } from "@/stores/editorStore";
import { useFlowStore } from "@/stores/flowStore";
import { cn } from "@/lib/utils";
import { useUiStore } from "@/stores/uiStore";
import { Header } from "./Header";
import { WelcomeScreen } from "./WelcomeScreen";

/** Matches `--dur-base` in index.css, plus a frame of slack. */
const PANE_ANIMATION_MS = 200;

/**
 * Collapses and expands a panel when `visible` changes, gliding while it does.
 * `animating` is true for the length of the glide; dragging is never animated.
 */
function useCollapsiblePanel(visible: boolean) {
  const panelRef = usePanelRef();
  const [animating, setAnimating] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const firstRun = useRef(true);
  /** Mirrors `animating` for callbacks, so a glide is never mistaken for a drag. */
  const gliding = useRef(false);

  // Follow `visible`. The first run sets the pane as restored, without the glide.
  useEffect(() => {
    const panel = panelRef.current;
    const instant = firstRun.current;
    firstRun.current = false;
    if (!panel || panel.isCollapsed() === !visible) return;
    if (!instant) {
      clearTimeout(timer.current);
      gliding.current = true;
      setAnimating(true);
      timer.current = setTimeout(() => {
        gliding.current = false;
        setAnimating(false);
      }, PANE_ANIMATION_MS);
    }
    if (visible) panel.expand();
    else panel.collapse();
  }, [visible, panelRef]);

  useEffect(() => () => clearTimeout(timer.current), []);

  /**
   * Keep the toggle in step when the pane is dragged shut or open. Reads the
   * panel's collapsed state rather than its width, which lags behind while
   * the pane glides and would flip the toggle straight back.
   */
  const syncVisibility = useCallback(() => {
    const panel = panelRef.current;
    if (!panel || gliding.current) return;
    const open = !panel.isCollapsed();
    const { responseVisible, setResponseVisible } = useUiStore.getState();
    if (open !== responseVisible) setResponseVisible(open);
  }, [panelRef]);

  return { panelRef, animating, syncVisibility };
}

const ResponsePane = memo(function ResponsePane({
  visible,
  animating,
  panelRef,
  onResize,
}: {
  visible: boolean;
  animating: boolean;
  panelRef: ReturnType<typeof usePanelRef>;
  onResize: () => void;
}) {
  // A hidden pane renders nothing once it has finished closing.
  const showContent = visible || animating;
  return (
    <ResizablePanel
      defaultSize="46%"
      minSize="280px"
      collapsible
      collapsedSize={0}
      panelRef={panelRef}
      onResize={onResize}
      style={{ overflow: "hidden" }}
    >
      <div
        className={cn(
          "h-full min-w-[280px] pr-3 pb-3 transition-[opacity,translate] duration-(--dur-base) ease-(--ease-snappy)",
          !visible && "translate-x-4 opacity-0",
        )}
      >
        <div className="h-full overflow-hidden rounded-xl border bg-card/70 shadow-lg shadow-shadow/20">
          {showContent && <ResponsePanel />}
        </div>
      </div>
    </ResizablePanel>
  );
});

/** Shown when the open file changed on disk while it had unsaved edits. */
function ConflictBanner({ path, conflict }: { path: string; conflict: Conflict }) {
  const deleted = conflict === "deleted";
  return (
    <div className="mx-4 mb-2 flex items-center gap-2 rounded-lg border border-warning/40 bg-warning/10 px-3 py-1.5 text-sm">
      <span className="min-w-0 flex-1 truncate">
        {deleted ? "This file was deleted on disk." : "This file changed on disk."} Your unsaved edits are kept.
      </span>
      {deleted ? (
        <Button size="sm" variant="ghost" onClick={() => discardChanges(path)}>
          Discard
        </Button>
      ) : (
        <Button size="sm" variant="ghost" onClick={() => void reloadFromDisk(path)}>
          Reload from disk
        </Button>
      )}
      <Button size="sm" variant="ghost" onClick={() => void saveScenario(path)}>
        {deleted ? "Save to recreate" : "Overwrite"}
      </Button>
    </div>
  );
}

/**
 * The request editor. It alone subscribes to the scenario, so typing
 * re-renders the editor and not the response pane beside it.
 */
function EditorPane({ path }: { path: ProjectPath }) {
  const scenario = useEditorStore((s) => s.scenario);
  const conflict = useEditorStore((s) => (s.path ? s.buffers[s.path]?.conflict : null));
  const editorVersion = useEditorStore((s) => s.editorVersion);
  if (!scenario) return null;
  return (
    <div className="flex h-full flex-col">
      {conflict && <ConflictBanner path={path} conflict={conflict} />}
      <div className="min-h-0 flex-1">
        <RequestEditor key={`${path}:${editorVersion}`} scenario={scenario} onChange={editScenario} />
      </div>
    </div>
  );
}

/** Editor and response side by side. Mounted fresh after the welcome screen, so the pane starts in the right state. */
function EditorLayout({ path }: { path: ProjectPath }) {
  const responseVisible = useUiStore((s) => s.responseVisible);
  const { panelRef, animating, syncVisibility } = useCollapsiblePanel(responseVisible);
  return (
    <ResizablePanelGroup orientation="horizontal" data-animating={animating || undefined}>
      <ResizablePanel minSize="320px">
        <EditorPane path={path} />
      </ResizablePanel>
      <ResizableHandle className={cn("bg-transparent", !responseVisible && "opacity-0")} />
      <ResponsePane visible={responseVisible} animating={animating} panelRef={panelRef} onResize={syncVisibility} />
    </ResizablePanelGroup>
  );
}

function EditorArea() {
  const path = useEditorStore((s) => s.path);
  const hasScenario = useEditorStore((s) => s.scenario !== null);
  if (!path || !hasScenario) return <WelcomeScreen />;
  return <EditorLayout path={path} />;
}

/** The request editor, the selected critter, or the open flow; the editor when either is gone. */
function MainArea() {
  const view = useUiStore((s) => s.view);
  const critter = useCritterStore(selectSelectedCritter);
  const flowCritterId = useFlowStore((s) => s.open?.critter);
  const flowCritter = useCritterStore((s) => s.critters.find((c) => c.id === flowCritterId) ?? null);
  if (view === "flow" && flowCritter) {
    return (
      <div className="h-full border-t">
        <FlowEditor critter={flowCritter} />
      </div>
    );
  }
  if (view === "critter" && critter) {
    return (
      <div className="h-full border-t">
        <CritterPage key={critter.id} critter={critter} />
      </div>
    );
  }
  return <EditorArea />;
}

/**
 * A soft glow drawn as a gradient. A `blur()` filter looks the same but is
 * re-rendered on every repaint beneath it, which makes WebKitGTK crawl.
 */
const glow = (color: string, opacity: number): CSSProperties => ({
  background: `radial-gradient(closest-side, ${color}, transparent)`,
  // Themes turn the glow up or down with `--glow-intensity`.
  opacity: `min(1, calc(${opacity} * var(--glow-intensity, 1)))`,
});

/** Everything right of the sidebar. */
export function Workspace() {
  return (
    <main className="relative flex h-full flex-col overflow-hidden">
      {/* The glow behind the title bar. */}
      <div
        className="pointer-events-none absolute -top-52 -left-14 h-96 w-[42rem]"
        style={glow("var(--color-glow-1)", 0.2)}
      />
      <div
        className="pointer-events-none absolute -top-44 right-8 h-80 w-[24rem]"
        style={glow("var(--color-glow-2)", 0.1)}
      />
      <Header />
      <div className="relative min-h-0 flex-1">
        <MainArea />
      </div>
    </main>
  );
}
