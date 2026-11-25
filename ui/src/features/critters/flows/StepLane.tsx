import { useRef, useState, type DragEvent, type KeyboardEvent } from "react";
import { ArrowDown } from "lucide-react";
import { editFlow } from "@/actions/flows";
import type { CritterSummary, Step } from "@/api/client";
import { insertionIndex, moveTarget } from "@/domain/dragInsert";
import type { Dataflow } from "@/domain/flowDataflow";
import { insertStep, insertTool, moveStep, removeStep, stepRequest } from "@/domain/flowEdit";
import { cn } from "@/lib/utils";
import { useFlowStore, type FlowRunState } from "@/stores/flowStore";
import { REQUEST_MIME, STEP_MIME, TOOL_MIME } from "./dragTypes";
import { StepCard, type StepPhase } from "./StepCard";
import { isToolId, TOOLS } from "./tools";

function phaseOf(index: number, run: FlowRunState): StepPhase {
  if (!run.running) return run.results[index] ? "done" : "idle";
  if (run.results[index]) return "done";
  return index === run.current ? "running" : "queued";
}

const isOurDrag = (event: DragEvent): boolean =>
  [REQUEST_MIME, TOOL_MIME, STEP_MIME].some((mime) => event.dataTransfer.types.includes(mime));

/** "access_token ↓": the values one step passes on to later ones. */
function PassingChips({ names }: { names: string[] }) {
  if (names.length === 0) return <div className="h-2" />;
  return (
    <div
      role="group"
      className="flex flex-wrap items-center gap-1 py-1 pl-7"
      aria-label={`Passes on ${names.join(", ")}`}
    >
      {names.map((name) => (
        <span
          key={name}
          className="flex items-center gap-0.5 rounded-full bg-muted px-1.5 font-mono text-2xs text-captured"
        >
          {name} <ArrowDown className="size-2.5" />
        </span>
      ))}
    </div>
  );
}

interface StepLaneProps {
  critter: CritterSummary;
  steps: Step[];
  dataflow: Dataflow;
}

/**
 * The flow's steps as cards, in order. Drop a request to insert it, drag a
 * card to move it; Alt+↑/↓ moves the focused card, Delete removes it.
 */
export function StepLane({ critter, steps, dataflow }: StepLaneProps) {
  const requests = useFlowStore((s) => s.requests);
  const run = useFlowStore((s) => s.run);
  const selected = useFlowStore((s) => s.selected);
  const select = useFlowStore((s) => s.select);
  const cards = useRef<(HTMLDivElement | null)[]>([]);
  const [dropAt, setDropAt] = useState<number | null>(null);

  const focusCard = (index: number) => requestAnimationFrame(() => cards.current[index]?.focus());

  const onDragOver = (event: DragEvent) => {
    if (!isOurDrag(event)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = event.dataTransfer.types.includes(STEP_MIME) ? "move" : "copy";
    const midpoints = steps.map((_, index) => {
      const rect = cards.current[index]?.getBoundingClientRect();
      return rect ? rect.top + rect.height / 2 : Number.POSITIVE_INFINITY;
    });
    setDropAt(insertionIndex(event.clientY, midpoints));
  };

  const onDrop = (event: DragEvent) => {
    const at = dropAt ?? steps.length;
    setDropAt(null);
    if (!isOurDrag(event)) return;
    event.preventDefault();
    const request = event.dataTransfer.getData(REQUEST_MIME);
    if (request) {
      editFlow((flow) => insertStep(flow, at, request));
      select(at);
      return;
    }
    const tool = event.dataTransfer.getData(TOOL_MIME);
    if (isToolId(tool)) {
      editFlow((flow) => insertTool(flow, at, TOOLS[tool].create()));
      select(at);
      return;
    }
    const from = Number(event.dataTransfer.getData(STEP_MIME));
    const to = Number.isNaN(from) ? null : moveTarget(from, at);
    if (to === null) return;
    editFlow((flow) => moveStep(flow, from, to));
    select(to);
  };

  const onCardKeyDown = (index: number) => (event: KeyboardEvent) => {
    if (event.target !== event.currentTarget) return;
    let to: number | null = null;
    if (event.altKey && event.key === "ArrowUp" && index > 0) to = index - 1;
    else if (event.altKey && event.key === "ArrowDown" && index < steps.length - 1) to = index + 1;
    if (to !== null) {
      event.preventDefault();
      const target = to;
      editFlow((flow) => moveStep(flow, index, target));
      select(target);
      focusCard(target);
      return;
    }
    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      editFlow((flow) => removeStep(flow, index));
      focusCard(Math.max(0, index - 1));
    } else if (event.key === "Enter") {
      event.preventDefault();
      select(index);
    } else if (!event.altKey && event.key === "ArrowDown") {
      event.preventDefault();
      focusCard(Math.min(index + 1, steps.length - 1));
    } else if (!event.altKey && event.key === "ArrowUp") {
      event.preventDefault();
      focusCard(Math.max(index - 1, 0));
    }
  };

  return (
    <div
      role="list"
      aria-label="Steps"
      onDragOver={onDragOver}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDropAt(null);
      }}
      onDrop={onDrop}
      className={cn("h-full overflow-y-auto p-3", dropAt !== null && "bg-accent/20")}
    >
      {steps.length === 0 ? (
        <p className="rounded-lg border border-dashed p-6 text-center text-sm text-muted-foreground">
          Drag requests or tools here, or use + in the list, to add steps.
        </p>
      ) : (
        steps.map((step, index) => (
          // Steps have no ids; their order is their identity, and moves re-render the lane.
          // oxlint-disable-next-line react/no-array-index-key
          <div key={index} role="listitem">
            <StepCard
              index={index}
              step={step}
              scenario={requests[stepRequest(step)]}
              result={run.results[index] ?? null}
              phase={phaseOf(index, run)}
              problems={dataflow.problems.filter((p) => p.step === index).map((p) => p.message)}
              selected={index === selected}
              shifted={dropAt !== null && index >= dropAt}
              critter={critter}
              cardRef={(card) => {
                cards.current[index] = card;
              }}
              onSelect={() => select(index)}
              onKeyDown={onCardKeyDown(index)}
            />
            {index < steps.length - 1 && <PassingChips names={dataflow.passing[index] ?? []} />}
          </div>
        ))
      )}
    </div>
  );
}
