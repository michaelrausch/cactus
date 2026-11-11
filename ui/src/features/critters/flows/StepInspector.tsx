import { useMemo, useState, type ReactNode } from "react";
import { ExternalLink, MousePointerClick, Plus, X } from "lucide-react";
import { editFlow } from "@/actions/flows";
import { selectScenario } from "@/actions/scenario";
import type { CritterSummary, FlowStep, HttpResponse, JsonValue, ProjectPath } from "@/api/client";
import { KeyValueTable } from "@/components/KeyValueTable";
import { KnownNamesContext } from "@/components/template/knownNames";
import { TemplateField } from "@/components/template/TemplateField";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { Dataflow, UseSource } from "@/domain/flowDataflow";
import {
  stepAssertRows,
  stepMapRows,
  updateStep,
  withCapture,
  withExpect,
  withOverride,
  withStepAssertRows,
  withStepMapRows,
  withWatch,
} from "@/domain/flowEdit";
import { captureNameFor } from "@/domain/jsonPick";
import { expressionSuggester, operatorSuggester, type Suggester } from "@/domain/suggestions";
import { JsonTree } from "@/features/response/JsonTree";
import { cn } from "@/lib/utils";
import { useFlowStore } from "@/stores/flowStore";

/** Statuses offered for Expect. */
const COMMON_STATUSES = [200, 201, 204, 401, 403, 404];

function Section({ title, hint, children }: { title: string; hint?: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-1.5">
      <h4 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">{title}</h4>
      {hint && <p className="-mt-1 text-xs text-muted-foreground">{hint}</p>}
      {children}
    </section>
  );
}

function sourceText(source: UseSource): string {
  switch (source.kind) {
    case "with":
      return "this step's With";
    case "step":
      return `captured by step ${source.step + 1}`;
    case "request":
      return "the request's variables";
    case "scope":
      return source.label;
    case "missing":
      return "not set";
  }
}

/** Each `{{name}}` the step uses and where its value comes from; a missing one can be added to With. */
function Uses({ dataflow, index, update }: { dataflow: Dataflow; index: number; update: Update }) {
  const uses = dataflow.uses[index] ?? [];
  if (uses.length === 0) return <p className="text-xs text-muted-foreground">No {"{{variables}}"}.</p>;
  return (
    <ul className="flex flex-col gap-0.5 text-xs">
      {uses.map(({ name, source }) => (
        <li key={name} className="flex items-center gap-2">
          <span className="font-mono">{`{{${name}}}`}</span>
          {source.kind === "missing" ? (
            <button
              type="button"
              onClick={() => update((step) => withOverride(step, name))}
              className="rounded text-warning underline-offset-2 focus-ring hover:underline"
            >
              not set · add to With
            </button>
          ) : (
            <span className={cn("text-muted-foreground", source.kind === "step" && "text-captured")}>
              {sourceText(source)}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}

/** The last response's body as JSON, if it is JSON. */
function parsedBody(response: HttpResponse | null): JsonValue | null {
  if (!response || response.truncated) return null;
  try {
    return JSON.parse(response.body) as JsonValue;
  } catch {
    return null;
  }
}

type Update = (change: (step: FlowStep) => FlowStep) => void;

function Captures({
  step,
  update,
  suggest,
  response,
}: {
  step: FlowStep;
  update: Update;
  suggest: Suggester;
  response: HttpResponse | null;
}) {
  const rows = useMemo(() => stepMapRows(step, "capture"), [step]);
  const [picking, setPicking] = useState(false);
  const body = useMemo(() => parsedBody(response), [response]);
  return (
    <>
      <KeyValueTable
        rows={rows}
        keyPlaceholder="access_token"
        valuePlaceholder="res.body.access_token"
        valueColumn={{ suggest }}
        onChange={(next) => update((s) => withStepMapRows(s, "capture", next))}
      />
      <Button
        size="sm"
        variant="ghost"
        className="self-start"
        disabled={body === null}
        title={body === null ? "Run the flow to pick from this step's response" : undefined}
        aria-pressed={picking}
        onClick={() => setPicking(!picking)}
      >
        <MousePointerClick /> {picking ? "Done picking" : "Pick from last response"}
      </Button>
      {picking && body !== null && (
        <div className="max-h-64 overflow-auto rounded-md border p-2">
          <JsonTree
            value={body}
            onPick={(expression) => update((s) => withCapture(s, captureNameFor(expression), expression))}
          />
        </div>
      )}
    </>
  );
}

function Checks({ step, update, suggest }: { step: FlowStep; update: Update; suggest: Suggester }) {
  const rows = useMemo(() => stepAssertRows(step), [step]);
  return (
    <KeyValueTable
      rows={rows}
      toggles
      keyPlaceholder="res.body.db"
      valuePlaceholder="eq ok"
      keyColumn={{ suggest }}
      valueColumn={{ suggest: operatorSuggester }}
      echo={(next) => stepAssertRows(withStepAssertRows(step, next))}
      onChange={(next) => update((s) => withStepAssertRows(s, next))}
    />
  );
}

/** Expressions compared with the previous scheduled run. */
function Watches({ step, update, suggest }: { step: FlowStep; update: Update; suggest: Suggester }) {
  const watch = step.watch ?? [];
  const [draft, setDraft] = useState("");
  const set = (list: string[]) => update((s) => withWatch(s, list));
  return (
    <div className="flex flex-col gap-1">
      {watch.map((expression, i) => (
        // Keyed by position: keying by the text would remount the field on every keystroke.
        // oxlint-disable-next-line react/no-array-index-key
        <div key={i} className="flex items-center gap-1 rounded-md border">
          <TemplateField
            value={expression}
            templates={false}
            suggest={suggest}
            aria-label={`Watched expression ${i + 1}`}
            className="flex-1"
            fieldClassName="h-8 px-2 font-mono text-xs"
            onChange={(value) => set(watch.map((w, j) => (j === i ? value : w)))}
          />
          <button
            type="button"
            aria-label={`Stop watching ${expression}`}
            onClick={() => set(watch.filter((_, j) => j !== i))}
            className="mr-1 grid size-6 place-items-center rounded text-muted-foreground focus-ring hover:text-foreground"
          >
            <X className="size-3.5" />
          </button>
        </div>
      ))}
      <form
        className="flex items-center gap-1 rounded-md border border-dashed"
        onSubmit={(event) => {
          event.preventDefault();
          if (!draft.trim()) return;
          set([...watch, draft]);
          setDraft("");
        }}
      >
        <TemplateField
          value={draft}
          templates={false}
          suggest={suggest}
          placeholder="res.body.version"
          aria-label="New watched expression"
          className="flex-1"
          fieldClassName="h-8 px-2 font-mono text-xs"
          onChange={setDraft}
        />
        <button
          type="submit"
          aria-label="Watch"
          className="mr-1 grid size-6 place-items-center rounded text-muted-foreground focus-ring hover:text-foreground"
        >
          <Plus className="size-3.5" />
        </button>
      </form>
    </div>
  );
}

function StepDetails({ index, step, dataflow }: { index: number; step: FlowStep; dataflow: Dataflow }) {
  const scenario = useFlowStore((s) => s.requests[step.request.trim()]);
  const response = useFlowStore((s) => s.run.results[index]?.response ?? null);
  const scope = useFlowStore((s) => s.scope);
  const suggest = useMemo(() => expressionSuggester(response), [response]);
  // What `{{name}}` can resolve to here: the critter's scope and everything the flow captures.
  const known = useMemo(
    () => [...new Set([...Object.keys(scope), ...dataflow.captures.flat(), ...Object.keys(step.with ?? {})])],
    [scope, dataflow.captures, step.with],
  );
  const update: Update = (change) => editFlow((flow) => updateStep(flow, index, change));
  const withRows = useMemo(() => stepMapRows(step, "with"), [step]);

  return (
    <KnownNamesContext value={known}>
      <div className="flex flex-col gap-5 p-4">
        <Section title={`Step ${index + 1}`}>
          <div className="flex items-center gap-2 text-sm">
            <span className="min-w-0 flex-1 truncate font-mono text-xs">{step.request || "No request"}</span>
            {scenario && (
              <Button size="sm" variant="ghost" onClick={() => void selectScenario(step.request.trim() as ProjectPath)}>
                <ExternalLink /> Open
              </Button>
            )}
          </div>
          {scenario === null && <p className="text-xs text-warning">This request doesn't exist or doesn't parse.</p>}
        </Section>

        <Section title="Expect">
          <Input
            type="number"
            list="flow-expect-statuses"
            className="h-8 w-28"
            placeholder="any"
            value={step.expect ?? ""}
            onChange={(event) => {
              const status = Number.parseInt(event.target.value, 10);
              update((s) => withExpect(s, Number.isNaN(status) ? null : status));
            }}
          />
          <datalist id="flow-expect-statuses">
            {COMMON_STATUSES.map((status) => (
              <option key={status} value={status} />
            ))}
          </datalist>
        </Section>

        <Section title="Uses">
          <Uses dataflow={dataflow} index={index} update={update} />
        </Section>

        <Section title="With">
          <KeyValueTable
            rows={withRows}
            keyPlaceholder="Variable"
            valueColumn={{ templates: true }}
            onChange={(next) => update((s) => withStepMapRows(s, "with", next))}
          />
        </Section>

        <Section title="Capture">
          <Captures step={step} update={update} suggest={suggest} response={response} />
        </Section>

        <Section title="Checks">
          <Checks step={step} update={update} suggest={suggest} />
        </Section>

        <Section title="Watch" hint="A scheduled run alerts when these change.">
          <Watches step={step} update={update} suggest={suggest} />
        </Section>
      </div>
    </KnownNamesContext>
  );
}

/** Everything about the selected step. */
export function StepInspector({ dataflow }: { critter: CritterSummary; dataflow: Dataflow }) {
  const selected = useFlowStore((s) => s.selected);
  const step = useFlowStore((s) => (s.selected === null ? undefined : s.model?.steps[s.selected]));
  return (
    <div className="h-full overflow-y-auto">
      {selected === null || !step ? (
        <p className="p-4 text-sm text-muted-foreground">Select a step to edit it.</p>
      ) : (
        <StepDetails key={selected} index={selected} step={step} dataflow={dataflow} />
      )}
    </div>
  );
}
