import { Bug } from "lucide-react";
import type { FuzzTarget, PayloadKind } from "@/api/client";
import { exportSources, setProblems } from "@/domain/findings";
import {
  bind,
  ChecklistInput,
  FailOnInput,
  Fields,
  NumberInput,
  RequestInput,
  RequestVariablesInput,
  type Rule,
  SetInput,
} from "./fields";
import { defineTool } from "./types";

/** Mirrors `fuzz::DEFAULT_CONCURRENCY`, `MAX_CONCURRENCY`, `DEFAULT_ITERATIONS` and `MAX_ITERATIONS` in Rust. */
const DEFAULT_CONCURRENCY = 4;
const MAX_CONCURRENCY = 32;
const DEFAULT_ITERATIONS = 20;
const MAX_ITERATIONS = 200;

const TARGETS: readonly { value: FuzzTarget; label: string }[] = [
  { value: "query", label: "Query string parameters" },
  { value: "path", label: "Path parameters (:id)" },
  { value: "headers", label: "Headers (not auth or cookies)" },
  { value: "body", label: "Body fields (JSON or form)" },
];
/** Mirrors `Fuzz::targets` in Rust. */
const DEFAULT_TARGETS: readonly FuzzTarget[] = ["query", "path", "body"];

const PAYLOADS: readonly { value: PayloadKind; label: string }[] = [
  { value: "sqli", label: "SQL injection" },
  { value: "xss", label: "Cross-site scripting (XSS)" },
  { value: "traversal", label: "Path traversal (../etc/passwd)" },
  { value: "command", label: "Shell command injection" },
  { value: "overflow", label: "Oversized and format-string text" },
  { value: "types", label: "Wrong types (null, [], -1…)" },
  { value: "mutate", label: "Random mutations of each value" },
];
const ALL_PAYLOADS = PAYLOADS.map((p) => p.value);
/** Mirrors `fuzz::EXPORTS` in Rust. */
const SOURCES = exportSources(["requests", "errors"], true);
/** What it checks; mirrors `fuzz::detect` and `no_response` in Rust. */
const RULES: readonly Rule[] = [
  { label: "A system file came back (path traversal)", severity: "critical" },
  { label: "A shell command ran (command injection)", severity: "critical" },
  { label: "A database error came back (SQL injection)", severity: "high" },
  { label: "Markup echoed back unescaped as HTML (XSS)", severity: "high" },
  { label: "A stack trace came back", severity: "medium" },
  { label: "A server error (5xx) the original didn't get", severity: "medium" },
  { label: "A sleep payload slowed the response by 5s+", severity: "medium" },
  { label: "No response: dropped or timed out", severity: "medium" },
];

export const fuzz = defineTool<"fuzz">({
  label: "Fuzz",
  icon: Bug,
  create: () => ({ tool: "fuzz" }),
  summary: (step) => step.request ?? "",
  dataflow: (step) => ({
    templates: Object.values(step.with ?? {}),
    sets: Object.keys(step.set ?? {}),
    problems: [
      ...(step.request?.trim() ? [] : ["No request"]),
      ...((step.concurrency ?? 0) > MAX_CONCURRENCY ? [`Send at most ${MAX_CONCURRENCY} at once`] : []),
      ...((step.iterations ?? 0) > MAX_ITERATIONS ? [`At most ${MAX_ITERATIONS} iterations`] : []),
      ...setProblems(step.set, SOURCES),
    ],
  }),
  Editor: (props) => {
    const mutates = !props.step.payloads?.length || props.step.payloads.includes("mutate");
    return (
      <Fields>
        <RequestInput label="Request to fuzz" {...bind(props, "request")} />
        <RequestVariablesInput {...bind(props, "with")} />
        <ChecklistInput
          label="Parts of the request to fuzz"
          options={TARGETS}
          defaults={DEFAULT_TARGETS}
          {...bind(props, "targets")}
        />
        <ChecklistInput
          label="Payloads to send"
          options={PAYLOADS}
          defaults={ALL_PAYLOADS}
          {...bind(props, "payloads")}
        />
        {mutates && (
          <>
            <NumberInput
              label="Random mutations per value"
              placeholder={String(DEFAULT_ITERATIONS)}
              max={MAX_ITERATIONS}
              {...bind(props, "iterations")}
            />
            <NumberInput
              label="Mutation seed (to repeat a run exactly)"
              placeholder="Random"
              {...bind(props, "seed")}
            />
          </>
        )}
        <NumberInput
          label="Requests in flight at once"
          placeholder={String(DEFAULT_CONCURRENCY)}
          max={MAX_CONCURRENCY}
          {...bind(props, "concurrency")}
        />
        <FailOnInput rules={RULES} {...bind(props, "fail_on")} />
        <SetInput sources={SOURCES} {...bind(props, "set")} />
      </Fields>
    );
  },
});
