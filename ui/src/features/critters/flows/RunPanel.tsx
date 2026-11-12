import { Check, CircleDashed, ExternalLink, Minus, X } from "lucide-react";
import { showStepResponse } from "@/actions/flows";
import type { CritterSummary, FlowStep, FlowStepResult, FlowStepStart, HttpResponse } from "@/api/client";
import { CountBadge } from "@/components/Badges";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { MethodBadge } from "@/components/MethodBadge";
import { StatusCode } from "@/components/StatusCode";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { flowRunSummary, stepFailure } from "@/domain/flowRun";
import { BodyViewer } from "@/features/response/BodyViewer";
import { ChecksList } from "@/features/response/ChecksList";
import { HeadersTable } from "@/features/response/HeadersTable";
import { ResponseSummary } from "@/features/response/ResponseSummary";
import { formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useFlowStore } from "@/stores/flowStore";

/** Where a step is in the run shown. */
type RowState = "idle" | "queued" | "running" | "passed" | "failed" | "skipped";

function rowState(index: number, running: boolean, current: number | null, result: FlowStepResult | null): RowState {
  if (result) {
    if (result.skipped) return "skipped";
    return result.ok ? "passed" : "failed";
  }
  if (!running) return "idle";
  return index === current ? "running" : "queued";
}

function StateIcon({ state, critter }: { state: RowState; critter: CritterSummary }) {
  switch (state) {
    case "running":
      return <CritterSprite species={critter.species} size={16} state="work" label="Sending" seed={critter.id} />;
    case "queued":
      return <CircleDashed className="size-4 text-muted-foreground" aria-label="Queued" />;
    case "passed":
      return <Check className="size-4 text-success" aria-label="Passed" />;
    case "failed":
      return <X className="size-4 text-danger" aria-label="Failed" />;
    case "skipped":
      return <Minus className="size-4 text-muted-foreground" aria-label="Skipped" />;
    case "idle":
      return <span className="size-4" />;
  }
}

interface RowProps {
  index: number;
  step: FlowStep;
  start: FlowStepStart | null;
  result: FlowStepResult | null;
  state: RowState;
  selected: boolean;
  critter: CritterSummary;
}

/** One request of the run: method, URL as sent, status and time. */
function RunRow({ index, step, start, result, state, selected, critter }: RowProps) {
  const select = useFlowStore((s) => s.select);
  const method = result?.method || start?.method || "";
  const url = result?.url || start?.url || "";
  const response = result?.response ?? null;
  return (
    <li>
      <button
        type="button"
        aria-current={selected || undefined}
        onClick={() => select(index)}
        className={cn(
          "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs focus-ring hover:bg-accent/50",
          selected && "bg-accent/70",
          (state === "queued" || state === "skipped") && "text-muted-foreground",
        )}
      >
        <StateIcon state={state} critter={critter} />
        <span className="w-4 shrink-0 text-muted-foreground tabular-nums">{index + 1}</span>
        {method && <MethodBadge method={method} className="w-12 shrink-0 text-2xs" />}
        <span className="min-w-0 flex-1 truncate font-mono" title={url || step.request}>
          {url || step.request}
        </span>
        {response && <StatusCode status={response.status} className="shrink-0 font-semibold tabular-nums" />}
        {response && (
          <span className="w-14 shrink-0 text-right text-muted-foreground tabular-nums">
            {formatDuration(response.timeMs)}
          </span>
        )}
        {state === "failed" && !response && <span className="shrink-0 text-danger">error</span>}
      </button>
    </li>
  );
}

function ResponseTabs({ response }: { response: HttpResponse }) {
  const failed = response.checks.filter((check) => !check.ok).length;
  const captured = Object.entries(response.captured);
  return (
    <Tabs defaultValue="body" className="min-h-0 flex-1 gap-3">
      <TabsList variant="line" className="h-8 gap-2">
        <TabsTrigger value="body">Body</TabsTrigger>
        <TabsTrigger value="headers">
          Headers
          <CountBadge count={response.headers.length} />
        </TabsTrigger>
        {response.checks.length > 0 && (
          <TabsTrigger value="checks">
            Checks
            <span className={cn("ml-1 text-xs tabular-nums", failed ? "text-danger" : "text-success")}>
              {response.checks.length - failed}/{response.checks.length}
            </span>
          </TabsTrigger>
        )}
        {captured.length > 0 && (
          <TabsTrigger value="captured">
            Captured
            <CountBadge count={captured.length} />
          </TabsTrigger>
        )}
      </TabsList>
      <TabsContent value="body" className="min-h-0">
        <BodyViewer response={response} />
      </TabsContent>
      <TabsContent value="headers" className="min-h-0 overflow-auto">
        <HeadersTable response={response} />
      </TabsContent>
      <TabsContent value="checks" className="min-h-0 overflow-auto">
        <ChecksList checks={response.checks} />
      </TabsContent>
      <TabsContent value="captured" className="min-h-0 overflow-auto">
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 font-mono text-xs">
          {captured.map(([name, value]) => (
            <div key={name} className="contents">
              <dt className="text-captured">{name}</dt>
              <dd className="truncate" title={value}>
                {value}
              </dd>
            </div>
          ))}
        </dl>
      </TabsContent>
    </Tabs>
  );
}

/** The selected step of the run: where it went and what came back. */
function StepResult({ step, start, result, state }: Omit<RowProps, "index" | "selected" | "critter">) {
  const method = result?.method || start?.method || "";
  const url = result?.url || start?.url || "";
  const response = result?.response ?? null;
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 border-t p-3">
      <div className="flex items-start gap-2">
        <p className="min-w-0 flex-1 font-mono text-xs break-all select-text">
          {method && <MethodBadge method={method} className="mr-2" />}
          {url || step.request || "No request"}
        </p>
        {response && result && (
          <Button size="sm" variant="ghost" className="h-7 shrink-0" onClick={() => void showStepResponse(result)}>
            <ExternalLink /> Open
          </Button>
        )}
      </div>
      {state === "queued" && <p className="text-xs text-muted-foreground">Queued</p>}
      {state === "running" && <p className="text-xs text-muted-foreground">Sending…</p>}
      {state === "skipped" && <p className="text-xs text-muted-foreground">Skipped</p>}
      {result && !response && !result.skipped && (
        <pre className="rounded-md border border-danger/30 bg-danger/5 p-2 font-mono text-xs whitespace-pre-wrap text-danger">
          {stepFailure(result)}
        </pre>
      )}
      {response && result && (
        <>
          <ResponseSummary response={response} historyAt={null} />
          {!result.ok && <p className="text-xs text-danger">{stepFailure(result)}</p>}
          <ResponseTabs key={`${result.index}:${response.timeMs}`} response={response} />
        </>
      )}
    </div>
  );
}

function RunHeading({ steps }: { steps: number }) {
  const running = useFlowStore((s) => s.run.running);
  const current = useFlowStore((s) => s.run.current);
  const last = useFlowStore((s) => s.run.last);
  const stale = useFlowStore((s) => s.run.stale);
  let text = "Not run";
  if (running) text = current === null ? "Running…" : `Step ${current + 1} of ${steps}…`;
  else if (last) text = flowRunSummary(last);
  return (
    <p
      className={cn(
        "truncate px-3 py-2 text-xs text-muted-foreground",
        !running && last && (last.ok ? "text-success" : "text-danger"),
        stale && "text-muted-foreground",
      )}
      title={text}
    >
      {text}
      {stale && " · edited since"}
    </p>
  );
}

/** The interactive run as it happens: each request's URL, status and time, and the selected one's response. */
export function RunPanel({ critter, steps }: { critter: CritterSummary; steps: FlowStep[] }) {
  const run = useFlowStore((s) => s.run);
  const selected = useFlowStore((s) => s.selected);
  const shown = selected === null ? undefined : steps[selected];
  const at = (index: number) => ({
    start: run.started[index] ?? null,
    result: run.results[index] ?? null,
    state: rowState(index, run.running, run.current, run.results[index] ?? null),
  });
  return (
    <div className="flex h-full flex-col">
      <RunHeading steps={steps.length} />
      <ul aria-label="Run" className="flex max-h-[45%] shrink-0 flex-col overflow-y-auto px-1 pb-2">
        {steps.map((step, index) => (
          <RunRow
            // A step is its place in the run; the same request can appear twice.
            // oxlint-disable-next-line react/no-array-index-key
            key={index}
            index={index}
            step={step}
            selected={index === selected}
            critter={critter}
            {...at(index)}
          />
        ))}
      </ul>
      {shown && selected !== null && <StepResult step={shown} {...at(selected)} />}
    </div>
  );
}
