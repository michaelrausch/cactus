import { useMemo, type KeyboardEvent } from "react";
import { Play, Redo2, Save, TriangleAlert, Undo2 } from "lucide-react";
import { showCritter } from "@/actions/critters";
import {
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
import { CritterSprite } from "@/components/critters/CritterSprite";
import { IconButton } from "@/components/IconButton";
import { Button } from "@/components/ui/button";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { flowDataflow, type Dataflow } from "@/domain/flowDataflow";
import { cn } from "@/lib/utils";
import { selectFlowDirty, useFlowStore, type FlowMode, type FlowPanel } from "@/stores/flowStore";
import { FlowYamlEditor } from "./FlowYamlEditor";
import { RequestPalette } from "./RequestPalette";
import { RunPanel } from "./RunPanel";
import { StepInspector } from "./StepInspector";
import { StepLane } from "./StepLane";
import { Triggers } from "./Triggers";

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

function FlowHeader({ model, critter, flowId }: { model: Flow; critter: CritterSummary; flowId: string }) {
  const dirty = useFlowStore(selectFlowDirty);
  const visual = useFlowStore((s) => s.mode === "visual");
  const canUndo = useFlowStore((s) => s.undo.length > 0);
  const canRedo = useFlowStore((s) => s.redo.length > 0);
  const running = useFlowStore((s) => s.run.running);
  return (
    <div className="flex items-center gap-2 border-b px-3 py-2">
      <button
        type="button"
        title={`Show ${critter.name}`}
        onClick={() => showCritter(critter.id)}
        className="flex shrink-0 items-center gap-1.5 rounded-md px-1 text-sm text-muted-foreground focus-ring hover:bg-accent/40 hover:text-foreground"
      >
        <CritterSprite species={critter.species} size={16} label={critter.name} seed={critter.id} />
        {critter.name}
      </button>
      <span className="text-muted-foreground/50">/</span>
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
      <Triggers critter={critter} flowId={flowId} model={model} editable={visual} />
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

/** The selected step's settings, or the run as it happens. */
function SidePanel({ critter, model, dataflow }: { critter: CritterSummary; model: Flow; dataflow: Dataflow }) {
  const panel = useFlowStore((s) => s.panel);
  const show = useFlowStore((s) => s.panelShown);
  const running = useFlowStore((s) => s.run.running);
  return (
    <Tabs value={panel} onValueChange={(value) => show(value as FlowPanel)} className="flex h-full flex-col gap-0">
      <TabsList variant="line" className="h-9 shrink-0 gap-2 border-b px-3">
        <TabsTrigger value="step">Step</TabsTrigger>
        <TabsTrigger value="run">
          Run
          {running && <span className="ml-1 size-1.5 animate-pulse rounded-full bg-brand" />}
        </TabsTrigger>
      </TabsList>
      <TabsContent value="step" className="min-h-0 flex-1">
        <StepInspector critter={critter} dataflow={dataflow} />
      </TabsContent>
      <TabsContent value="run" className="min-h-0 flex-1">
        <RunPanel critter={critter} steps={model.steps} />
      </TabsContent>
    </Tabs>
  );
}

/** Palette, step lane and step or run pane, or the flow's YAML. Ctrl+S saves, Ctrl+Enter runs. */
export function FlowEditor({ critter }: { critter: CritterSummary }) {
  const model = useFlowStore((s) => s.model);
  const flowId = useFlowStore((s) => s.open?.flow);
  const mode = useFlowStore((s) => s.mode);
  const requests = useFlowStore((s) => s.requests);
  const scope = useFlowStore((s) => s.scope);
  const dataflow = useMemo(
    () => (model ? flowDataflow(model.steps, requests, scope, critter.name) : null),
    [model, requests, scope, critter.name],
  );
  if (!model || !dataflow || !flowId) return null;

  const onKeyDown = (event: KeyboardEvent) => {
    if (!(event.ctrlKey || event.metaKey) || mode !== "visual" || isTextField(event.target)) return;
    const key = event.key.toLowerCase();
    if (key === "z" && !event.shiftKey) undoFlowEdit();
    else if ((key === "z" && event.shiftKey) || key === "y") redoFlowEdit();
    else return;
    event.preventDefault();
  };

  return (
    <div className="flex h-full min-h-96 flex-col" onKeyDown={onKeyDown}>
      <FlowHeader model={model} critter={critter} flowId={flowId} />
      <ConflictBanner />
      <div className="min-h-0 flex-1">
        {/* Remounted per mode, as the panels differ. */}
        <ResizablePanelGroup key={mode} orientation="horizontal">
          {mode === "visual" ? (
            <>
              <ResizablePanel defaultSize="22%" minSize="160px">
                <RequestPalette />
              </ResizablePanel>
              <ResizableHandle />
              <ResizablePanel defaultSize="34%" minSize="220px">
                <StepLane critter={critter} steps={model.steps} dataflow={dataflow} />
              </ResizablePanel>
            </>
          ) : (
            <ResizablePanel defaultSize="56%" minSize="320px">
              <FlowYamlEditor />
            </ResizablePanel>
          )}
          <ResizableHandle />
          <ResizablePanel defaultSize="44%" minSize="280px">
            <SidePanel critter={critter} model={model} dataflow={dataflow} />
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
      <ProblemsFooter dataflow={dataflow} steps={model.steps.length} />
    </div>
  );
}
