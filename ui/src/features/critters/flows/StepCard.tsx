import { useState, type DragEvent, type KeyboardEvent, type Ref } from "react";
import { Check, ChevronRight, CircleDashed, ExternalLink, Minus, X } from "lucide-react";
import { showStepResponse } from "@/actions/flows";
import type { CritterSummary, FlowStepResult, Scenario, Step, ToolOutput } from "@/api/client";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { MethodBadge } from "@/components/MethodBadge";
import { StatusCode } from "@/components/StatusCode";
import { Button } from "@/components/ui/button";
import { isToolStep } from "@/domain/flowEdit";
import { stepFailure } from "@/domain/flowRun";
import { formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";
import { STEP_MIME } from "./dragTypes";
import { toolOf } from "./tools";

/** Where a step is in a run. */
export type StepPhase = "idle" | "queued" | "running" | "done";

interface StepCardProps {
  index: number;
  step: Step;
  /** Its request: loaded, null when missing, undefined while loading. */
  scenario: Scenario | null | undefined;
  result: FlowStepResult | null;
  phase: StepPhase;
  problems: string[];
  selected: boolean;
  /** Moved aside to make room for a drop. */
  shifted: boolean;
  critter: CritterSummary;
  cardRef: Ref<HTMLDivElement>;
  onSelect: () => void;
  onKeyDown: (event: KeyboardEvent) => void;
}

function Outcome({
  phase,
  verdict,
  critter,
}: {
  phase: StepPhase;
  verdict: FlowStepResult | null;
  critter: CritterSummary;
}) {
  if (phase === "queued") return <CircleDashed className="size-4 text-muted-foreground" aria-label="Queued" />;
  if (phase === "running") {
    return <CritterSprite species={critter.species} size={16} state="work" label={`${critter.name} is sending`} />;
  }
  if (!verdict) return null;
  if (verdict.skipped) return <Minus className="size-4 text-muted-foreground" aria-label="Skipped" />;
  return verdict.ok ? (
    <Check className="size-4 text-success" aria-label="Passed" />
  ) : (
    <X className="size-4 text-danger" aria-label="Failed" />
  );
}

/** What a tool step did: its status, text and the values it set. */
export function ToolResult({ output }: { output: ToolOutput }) {
  const values = Object.entries(output.values ?? {});
  return (
    <div className="flex flex-col gap-1.5 text-xs">
      {output.status !== undefined && <StatusCode status={output.status} className="font-semibold" />}
      {output.text && <p className="break-words whitespace-pre-wrap">{output.text}</p>}
      {values.map(([name, value]) => (
        <p key={name} className="truncate font-mono text-muted-foreground">
          {name} = {value}
        </p>
      ))}
    </div>
  );
}

/** What a finished step did: status, time, captures, failed checks and watched values. */
function ResultDetails({ result }: { result: FlowStepResult }) {
  const response = result.response;
  if (result.skipped) return <p className="text-xs text-muted-foreground">Skipped: an earlier step failed.</p>;
  if (result.output) return <ToolResult output={result.output} />;
  if (!response) return <p className="text-xs text-danger">{result.error ?? stepFailure(result)}</p>;
  const captured = Object.entries(response.captured);
  const failed = response.checks.filter((check) => !check.ok);
  const watched = Object.entries(result.watched);
  return (
    <div className="flex flex-col gap-1.5 text-xs">
      <div className="flex items-center gap-2">
        <StatusCode status={response.status} className="font-semibold" />
        <span className="text-muted-foreground tabular-nums">{formatDuration(response.timeMs)}</span>
        {response.relogged && <span className="text-info">logged in again</span>}
      </div>
      {failed.map((check) => (
        <p key={check.name} className="text-danger">
          {check.name}: {check.detail}
        </p>
      ))}
      {captured.length > 0 && (
        <p className="truncate text-muted-foreground">Captured {captured.map(([name]) => name).join(", ")}</p>
      )}
      {watched.map(([expression, value]) => (
        <p key={expression} className="truncate font-mono text-muted-foreground">
          {expression} = {value ?? "—"}
        </p>
      ))}
      <Button size="sm" variant="ghost" className="self-start" onClick={() => void showStepResponse(result)}>
        <ExternalLink /> Open in response viewer
      </Button>
    </div>
  );
}

/** One step: number, method, title and run state; drag to move, Enter to inspect. */
export function StepCard({
  index,
  step,
  scenario,
  result,
  phase,
  problems,
  selected,
  shifted,
  critter,
  cardRef,
  onSelect,
  onKeyDown,
}: StepCardProps) {
  const [expanded, setExpanded] = useState(false);
  const tool = isToolStep(step) ? toolOf(step) : null;
  const summary = isToolStep(step) ? (toolOf(step).summary?.(step).trim() ?? "") : "";
  let title: string;
  let hover: string;
  if (isToolStep(step)) {
    title = toolOf(step).label;
    hover = summary || title;
  } else {
    title = scenario?.info.name || step.request || "No request";
    hover = step.request;
  }
  const verdict = result;
  const failed = phase === "done" && verdict !== null && !verdict.ok && !verdict.skipped;
  const failure = result ? stepFailure(result) : "";
  /** critters.css pulses a running card and shakes a failed one. */
  let outcome: "running" | "fail" | undefined;
  if (phase === "running") outcome = "running";
  else if (failed) outcome = "fail";
  const onDragStart = (event: DragEvent) => {
    event.dataTransfer.setData(STEP_MIME, String(index));
    event.dataTransfer.setData("text/plain", hover);
    event.dataTransfer.effectAllowed = "move";
  };
  return (
    <div
      ref={cardRef}
      role="group"
      aria-roledescription="step"
      tabIndex={0}
      draggable
      onDragStart={onDragStart}
      onClick={onSelect}
      onKeyDown={onKeyDown}
      aria-label={`Step ${index + 1}: ${title}`}
      aria-current={selected || undefined}
      data-outcome={outcome}
      className={cn(
        "flow-card flex flex-col gap-2 rounded-lg border bg-card p-2.5 text-left shadow-sm focus-ring transition-[translate,border-color,opacity] duration-(--dur-fast) ease-(--ease-snappy)",
        selected && "border-brand-2/70",
        phase === "queued" && "opacity-50",
        shifted && "translate-y-3",
        failed && "border-danger/60",
      )}
    >
      <div className="flex items-center gap-2">
        <span className="w-5 shrink-0 text-xs text-muted-foreground tabular-nums">{index + 1}.</span>
        {tool && (
          <span className="grid w-10 shrink-0 place-items-center text-muted-foreground">
            <tool.icon className="size-3.5" />
          </span>
        )}
        {scenario && !tool && <MethodBadge method={scenario.http.method} className="w-10 shrink-0 text-2xs" />}
        <span className="flex min-w-0 flex-1 items-baseline gap-2 truncate" title={hover}>
          <span className={cn("truncate text-sm font-medium", summary && "shrink-0")}>{title}</span>
          {summary && <span className="truncate text-xs text-muted-foreground">{summary}</span>}
        </span>
        {problems.length > 0 && (
          <span className="size-2 shrink-0 rounded-full bg-warning" title={problems.join("\n")}>
            <span className="sr-only">{problems.join(", ")}</span>
          </span>
        )}
        {!isToolStep(step) && step.expect !== undefined && (
          <span className="shrink-0 rounded bg-muted px-1 text-2xs text-muted-foreground">expect {step.expect}</span>
        )}
        <Outcome phase={phase} verdict={verdict} critter={critter} />
        {verdict && phase === "done" && (
          <button
            type="button"
            aria-expanded={expanded}
            aria-label={expanded ? "Hide result" : "Show result"}
            onClick={(event) => {
              event.stopPropagation();
              setExpanded(!expanded);
            }}
            className="rounded focus-ring"
          >
            <ChevronRight className={cn("size-3.5 transition-transform", expanded && "rotate-90")} />
          </button>
        )}
      </div>
      {failed && !expanded && (
        <p className="truncate pl-7 text-xs text-danger" title={failure}>
          {failure}
        </p>
      )}
      {expanded && result && <ResultDetails result={result} />}
    </div>
  );
}
