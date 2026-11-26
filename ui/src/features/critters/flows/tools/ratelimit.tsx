import { Gauge } from "lucide-react";
import { exportSources, setProblems } from "@/domain/findings";
import {
  bind,
  Checkbox,
  FailOnInput,
  Fields,
  NumberInput,
  RequestInput,
  RequestVariablesInput,
  type Rule,
  SetInput,
} from "./fields";
import { defineTool } from "./types";

/** Mirrors `ratelimit::MAX_REQUESTS` and `MAX_CONCURRENCY` in Rust. */
const MAX_REQUESTS = 1000;
const MAX_CONCURRENCY = 100;
const DEFAULT_REQUESTS = 100;
/** Mirrors `ratelimit::EXPORTS` in Rust. */
const SOURCES = exportSources(["allowed", "sent", "limited_at", "rps"], true);

/** Mirrors `Ratelimit::requests` in Rust. */
const requests = (step: { limit?: number; requests?: number }) =>
  step.requests || (step.limit ? Math.min(step.limit * 3, MAX_REQUESTS) : DEFAULT_REQUESTS);

/** What it checks; mirrors the findings of `ratelimit::analyse` and the reset check in Rust. */
const rules = (step: { limit?: number; reset?: boolean }): Rule[] => [
  { label: "No request was ever refused with a 429", severity: "high" },
  ...(step.limit
    ? [{ label: `More than ${step.limit} requests got through before a 429`, severity: "medium" as const }]
    : []),
  { label: "A 429 didn't say when to retry (no Retry-After)", severity: "low" },
  { label: "A request succeeded after a 429", severity: "low" },
  ...(step.reset ? [{ label: "Still refused after waiting for Retry-After", severity: "low" as const }] : []),
];

export const ratelimit = defineTool<"ratelimit">({
  label: "Rate limit",
  icon: Gauge,
  create: () => ({ tool: "ratelimit" }),
  summary: (step) => [step.request ?? "", step.limit ? `≤ ${step.limit}` : ""].filter(Boolean).join(" "),
  dataflow: (step) => ({
    templates: Object.values(step.with ?? {}),
    sets: Object.keys(step.set ?? {}),
    problems: [
      ...(step.request?.trim() ? [] : ["No request"]),
      ...(requests(step) > MAX_REQUESTS ? [`Send at most ${MAX_REQUESTS} requests`] : []),
      ...(requests(step) <= (step.limit ?? 0) ? ["Send more requests than the limit"] : []),
      ...((step.concurrency ?? 0) > MAX_CONCURRENCY ? [`Send at most ${MAX_CONCURRENCY} at once`] : []),
      ...setProblems(step.set, SOURCES),
    ],
  }),
  Editor: (props) => (
    <Fields>
      <RequestInput label="Request to send repeatedly" {...bind(props, "request")} />
      <RequestVariablesInput {...bind(props, "with")} />
      <NumberInput
        label="Most requests that should get through before a 429"
        placeholder="Any"
        {...bind(props, "limit")}
      />
      <NumberInput
        label="Requests to send in total"
        placeholder={String(requests(props.step))}
        max={MAX_REQUESTS}
        {...bind(props, "requests")}
      />
      <NumberInput
        label="Requests in flight at once"
        placeholder="1"
        max={MAX_CONCURRENCY}
        {...bind(props, "concurrency")}
      />
      <Checkbox
        label="Once refused, wait for Retry-After and check a request gets through again"
        {...bind(props, "reset")}
      />
      <FailOnInput rules={rules(props.step)} {...bind(props, "fail_on")} />
      <SetInput sources={SOURCES} {...bind(props, "set")} />
    </Fields>
  ),
});
