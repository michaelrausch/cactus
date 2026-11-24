import { useState, type DragEvent, type KeyboardEvent, type Ref } from "react";
import { Check, ChevronRight, CircleDashed, ExternalLink, Minus, X } from "lucide-react";
import { showStepResponse } from "@/actions/flows";
import type { CritterSummary, FlowStep, FlowStepResult, Scenario } from "@/api/client";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { MethodBadge } from "@/components/MethodBadge";
import { StatusCode } from "@/components/StatusCode";
import { Button } from "@/components/ui/button";
import { stepFailure } from "@/domain/flowRun";
import { formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";
import { STEP_MIME } from "./dragTypes";

/** Where a step is in a run. */
export type StepPhase = "idle" | "queued" | "running" | "done";

interface StepCardProps {
  index: number;
  step: FlowStep;
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
  result,
  critter,
}: {
  phase: StepPhase;
  result: FlowStepResult | null;
  critter: CritterSummary;
}) {
  if (phase === "queued") return <CircleDashed className="size-4 text-muted-foreground" aria-label="Queued" />;
  if (phase === "running") {
    return <CritterSprite species={critter.species} size={16} state="work" label={`${critter.name} is sending`} />;
  }
  if (!result) return null;
  if (result.skipped) return <Minus className="size-4 text-muted-foreground" aria-label="Skipped" />;
  return result.ok ? (
    <Check className="size-4 text-success" aria-label="Passed" />
  ) : (
    <X className="size-4 text-danger" aria-label="Failed" />
  );
}

/** What a finished step did: status, time, captures, failed checks and watched values. */
function ResultDetails({ result }: { result: FlowStepResult }) {
  const response = result.response;
  if (result.skipped) return <p className="text-xs text-muted-foreground">Skipped: an earlier step failed.</p>;
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
  const title = scenario?.info.name || step.request || "No request";
  const failed = phase === "done" && result !== null && !result.ok && !result.skipped;
  /** critters.css pulses a running card and shakes a failed one. */
  let outcome: "running" | "fail" | undefined;
  if (phase === "running") outcome = "running";
  else if (failed) outcome = "fail";
  const onDragStart = (event: DragEvent) => {
    event.dataTransfer.setData(STEP_MIME, String(index));
    event.dataTransfer.setData("text/plain", step.request);
    event.dataTransfer.effectAllowed = "move";
  };
  return (
    <div
      ref={cardRef}
      role="listitem"
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
        {scenario && <MethodBadge method={scenario.http.method} className="w-10 shrink-0 text-2xs" />}
        <span className="min-w-0 flex-1 truncate text-sm font-medium" title={step.request}>
          {title}
        </span>
        {problems.length > 0 && (
          <span className="size-2 shrink-0 rounded-full bg-warning" title={problems.join("\n")}>
            <span className="sr-only">{problems.join(", ")}</span>
          </span>
        )}
        {step.expect !== undefined && (
          <span className="shrink-0 rounded bg-muted px-1 text-2xs text-muted-foreground">expect {step.expect}</span>
        )}
        <Outcome phase={phase} result={result} critter={critter} />
        {result && phase === "done" && (
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
      {failed && !expanded && result && (
        <p className="truncate pl-7 text-xs text-danger" title={stepFailure(result)}>
          {stepFailure(result)}
        </p>
      )}
      {expanded && result && <ResultDetails result={result} />}
    </div>
  );
}
