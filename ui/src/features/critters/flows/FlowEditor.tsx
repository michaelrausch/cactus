import { useMemo, type KeyboardEvent } from "react";
import { ArrowLeft, Play, Redo2, Save, TriangleAlert, Undo2 } from "lucide-react";
import {
  closeFlow,
  editFlow,
  overwriteFlow,
  redoFlowEdit,
  reloadFlow,
  runOpenFlow,
  saveOpenFlow,
  setFlowMode,
  undoFlowEdit,
} from "@/actions/flows";
import type { CritterSummary, Flow } from "@/api/client";
import { withShortcut } from "@/app/shortcuts";
import { IconButton } from "@/components/IconButton";
import { Button } from "@/components/ui/button";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { flowDataflow, type Dataflow } from "@/domain/flowDataflow";
import { flowRunSummary } from "@/domain/flowRun";
import { FLOW_EDITOR_ATTRIBUTE } from "@/hooks/useKeyboardShortcuts";
import { cn } from "@/lib/utils";
import { selectFlowDirty, useFlowStore, type FlowMode } from "@/stores/flowStore";
import { FlowYamlEditor } from "./FlowYamlEditor";
import { RequestPalette } from "./RequestPalette";
import { StepInspector } from "./StepInspector";
import { StepLane } from "./StepLane";

const MODES: { mode: FlowMode; label: string }[] = [
  { mode: "visual", label: "Visual" },
  { mode: "yaml", label: "YAML" },
];

/** Typing in a field keeps its own undo; Ctrl+Z undoes flow edits everywhere else. */
const isTextField = (target: EventTarget): boolean =>
  target instanceof HTMLElement &&
  (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

function ModeSwitch() {
  const mode = useFlowStore((s) => s.mode);
  return (
    <div role="tablist" aria-label="Editor" className="flex rounded-lg bg-muted p-0.5">
      {MODES.map((option) => (
        <button
          key={option.mode}
          type="button"
          role="tab"
          aria-selected={mode === option.mode}
          onClick={() => void setFlowMode(option.mode)}
          className={cn(
            "rounded-md px-2.5 py-0.5 text-xs text-muted-foreground focus-ring",
            mode === option.mode && "bg-background text-foreground shadow-sm",
          )}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

function FlowHeader({ model }: { model: Flow }) {
  const dirty = useFlowStore(selectFlowDirty);
  const visual = useFlowStore((s) => s.mode === "visual");
  const canUndo = useFlowStore((s) => s.undo.length > 0);
  const canRedo = useFlowStore((s) => s.redo.length > 0);
  const running = useFlowStore((s) => s.run.running);
  return (
    <div className="flex items-center gap-2 border-b px-3 py-2">
      <IconButton label="Back to flows" onClick={closeFlow}>
        <ArrowLeft />
      </IconButton>
      <input
        value={model.name}
        disabled={!visual}
        aria-label="Flow name"
        placeholder="Untitled flow"
        onChange={(event) => editFlow((flow) => ({ ...flow, name: event.target.value }))}
        className="min-w-0 flex-1 rounded-md bg-transparent px-1.5 text-sm font-semibold outline-none hover:bg-accent/40 focus-visible:bg-accent/60 disabled:hover:bg-transparent"
      />
      {dirty && (
        <span className="size-2 shrink-0 rounded-full bg-brand" title="Unsaved changes">
          <span className="sr-only">Unsaved changes</span>
        </span>
      )}
      <ModeSwitch />
      <IconButton label="Undo (Ctrl+Z)" onClick={undoFlowEdit} disabled={!visual || !canUndo}>
        <Undo2 />
      </IconButton>
      <IconButton label="Redo (Ctrl+Shift+Z)" onClick={redoFlowEdit} disabled={!visual || !canRedo}>
        <Redo2 />
      </IconButton>
      <IconButton label={withShortcut("Save", "save")} onClick={() => void saveOpenFlow()} active={dirty}>
        <Save />
      </IconButton>
      <Button size="sm" onClick={runOpenFlow} disabled={running} title={withShortcut("Run", "send")}>
        <Play /> {running ? "Running…" : "Run"}
      </Button>
    </div>
  );
}

/** "3/3 passed · 1.24s" or the first failure, once a run finishes. */
function RunSummary() {
  const last = useFlowStore((s) => s.run.last);
  const stale = useFlowStore((s) => s.run.stale);
  if (!last) return null;
  return (
    <p
      className={cn(
        "border-b px-4 py-1.5 text-xs",
        last.ok ? "text-success" : "text-danger",
        stale && "text-muted-foreground",
      )}
    >
      {flowRunSummary(last)}
      {stale && " · edited since, may be stale"}
    </p>
  );
}

function ConflictBanner() {
  const conflict = useFlowStore((s) => s.conflict);
  if (!conflict) return null;
  return (
    <div className="flex items-center gap-2 border-b border-warning/40 bg-warning/10 px-3 py-1.5 text-sm">
      <span className="min-w-0 flex-1 truncate">This flow changed on disk. Your unsaved edits are kept.</span>
      <Button size="sm" variant="ghost" onClick={() => void reloadFlow()}>
        Reload from disk
      </Button>
      <Button size="sm" variant="ghost" onClick={() => void overwriteFlow()}>
        Overwrite
      </Button>
    </div>
  );
}

/** "2 problems": clicking selects the first step with one. */
function ProblemsFooter({ dataflow, steps }: { dataflow: Dataflow; steps: number }) {
  const select = useFlowStore((s) => s.select);
  const problems = dataflow.problems;
  return (
    <footer className="flex items-center gap-3 border-t px-4 py-1.5 text-xs text-muted-foreground">
      <span>
        {steps} {steps === 1 ? "step" : "steps"}
      </span>
      {problems.length > 0 && (
        <button
          type="button"
          onClick={() => select(problems[0]?.step ?? null)}
          title={problems.map((p) => `Step ${p.step + 1}: ${p.message}`).join("\n")}
          className="flex items-center gap-1 rounded text-warning focus-ring hover:underline"
        >
          <TriangleAlert className="size-3.5" />
          {problems.length} {problems.length === 1 ? "problem" : "problems"}
        </button>
      )}
    </footer>
  );
}

/** Palette, step lane and inspector, or the flow's YAML. Ctrl+S saves, Ctrl+Enter runs. */
export function FlowEditor({ critter }: { critter: CritterSummary }) {
  const model = useFlowStore((s) => s.model);
  const mode = useFlowStore((s) => s.mode);
  const requests = useFlowStore((s) => s.requests);
  const scope = useFlowStore((s) => s.scope);
  const dataflow = useMemo(
    () => (model ? flowDataflow(model.steps, requests, scope, critter.name) : null),
    [model, requests, scope, critter.name],
  );
  if (!model || !dataflow) return null;

  const onKeyDown = (event: KeyboardEvent) => {
    if (!(event.ctrlKey || event.metaKey) || mode !== "visual" || isTextField(event.target)) return;
    const key = event.key.toLowerCase();
    if (key === "z" && !event.shiftKey) undoFlowEdit();
    else if ((key === "z" && event.shiftKey) || key === "y") redoFlowEdit();
    else return;
    event.preventDefault();
  };

  return (
    <div {...{ [FLOW_EDITOR_ATTRIBUTE]: "" }} className="flex h-full min-h-96 flex-col" onKeyDown={onKeyDown}>
      <FlowHeader model={model} />
      <ConflictBanner />
      <RunSummary />
      <div className="min-h-0 flex-1">
        {mode === "visual" ? (
          <ResizablePanelGroup orientation="horizontal">
            <ResizablePanel defaultSize="26%" minSize="160px">
              <RequestPalette />
            </ResizablePanel>
            <ResizableHandle />
            <ResizablePanel defaultSize="38%" minSize="220px">
              <StepLane critter={critter} steps={model.steps} dataflow={dataflow} />
            </ResizablePanel>
            <ResizableHandle />
            <ResizablePanel defaultSize="36%" minSize="240px">
              <StepInspector critter={critter} dataflow={dataflow} />
            </ResizablePanel>
          </ResizablePanelGroup>
        ) : (
          <FlowYamlEditor />
        )}
      </div>
      <ProblemsFooter dataflow={dataflow} steps={model.steps.length} />
    </div>
  );
}
