import { Timer } from "lucide-react";
import { exportSources, setProblems } from "@/domain/findings";
import { wordSuggester } from "@/domain/suggestions";
import { bind, ChecksInput, Fields, NumberInput, RequestInput, RequestVariablesInput, SetInput } from "./fields";
import { defineTool } from "./types";

/** Mirrors `loadtest::MAX_REQUESTS`, `MAX_CONCURRENCY` and the defaults, and `wait::MAX_SECONDS`, in Rust. */
const MAX_REQUESTS = 10_000;
const MAX_CONCURRENCY = 100;
const MAX_SECONDS = 600;
const DEFAULT_REQUESTS = 100;
const DEFAULT_CONCURRENCY = 10;
/** The metrics `loadtest` reports; mirrors `loadtest::EXPORTS` in Rust. */
const METRIC_NAMES = ["p95", "p50", "p90", "p99", "min", "max", "mean", "rps", "error_rate", "non2xx", "requests"];
const METRICS = wordSuggester(METRIC_NAMES);
const SOURCES = exportSources(METRIC_NAMES, false);

export const loadtest = defineTool<"loadtest">({
  label: "Load test",
  icon: Timer,
  create: () => ({ tool: "loadtest" }),
  summary: (step) =>
    [step.request ?? "", step.duration ? `${step.duration}s` : `×${step.requests || DEFAULT_REQUESTS}`].join(" "),
  dataflow: (step) => ({
    templates: [...Object.values(step.with ?? {}), ...(step.checks ?? []).map((c) => c.value ?? "")],
    sets: Object.keys(step.set ?? {}),
    problems: [
      ...(step.request?.trim() ? [] : ["No request"]),
      ...((step.requests ?? 0) > MAX_REQUESTS ? [`Send at most ${MAX_REQUESTS} requests`] : []),
      ...((step.duration ?? 0) > MAX_SECONDS ? [`Run for at most ${MAX_SECONDS} seconds`] : []),
      ...((step.concurrency ?? 0) > MAX_CONCURRENCY ? [`Send at most ${MAX_CONCURRENCY} at once`] : []),
      ...setProblems(step.set, SOURCES),
    ],
  }),
  Editor: (props) => (
    <Fields>
      <RequestInput label="Request to load test" {...bind(props, "request")} />
      <RequestVariablesInput {...bind(props, "with")} />
      <NumberInput
        label="Requests to send in total"
        placeholder={props.step.duration ? "No limit" : String(DEFAULT_REQUESTS)}
        max={MAX_REQUESTS}
        {...bind(props, "requests")}
      />
      <NumberInput
        label="Or keep sending for this many seconds"
        placeholder="Off"
        max={MAX_SECONDS}
        {...bind(props, "duration")}
      />
      <NumberInput
        label="Requests in flight at once"
        placeholder={String(DEFAULT_CONCURRENCY)}
        max={MAX_CONCURRENCY}
        {...bind(props, "concurrency")}
      />
      <ChecksInput
        label="Fail the step unless these metrics pass (e.g. p95 lt 500)"
        keyPlaceholder="p95"
        suggest={METRICS}
        {...bind(props, "checks")}
      />
      <SetInput sources={SOURCES} {...bind(props, "set")} />
    </Fields>
  ),
});
