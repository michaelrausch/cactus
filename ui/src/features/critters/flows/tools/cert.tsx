import { ShieldCheck } from "lucide-react";
import { exportSources, setProblems } from "@/domain/findings";
import { wordSuggester } from "@/domain/suggestions";
import { bind, ChecksInput, FailOnInput, Fields, NumberInput, type Rule, SetInput, TextInput } from "./fields";
import { defineTool } from "./types";

/** Mirrors `cert::DEFAULT_WARN_DAYS` in Rust. */
const DEFAULT_WARN_DAYS = 30;
/** The metrics `cert` reports; mirrors `cert::EXPORTS` in Rust. */
const METRIC_NAMES = ["days_left", "handshake_ms"];
const METRICS = wordSuggester(METRIC_NAMES);
const SOURCES = exportSources(METRIC_NAMES, true);
/** What it checks; mirrors `cert::inspect` and the handshake checks in Rust. */
const RULES: readonly Rule[] = [
  { label: "Expired, or not valid yet", severity: "critical" },
  { label: "Expires within 7 days", severity: "high" },
  { label: "Expires within the days above", severity: "medium" },
  { label: "Not signed by a trusted authority", severity: "high" },
  { label: "Not issued for this host name", severity: "high" },
  { label: "Weak key (RSA < 2048) or signature (MD5, SHA-1)", severity: "high" },
  { label: "Server doesn't use TLS 1.3", severity: "info" },
];

export const cert = defineTool<"cert">({
  label: "Certificate",
  icon: ShieldCheck,
  create: () => ({ tool: "cert" }),
  summary: (step) => step.url ?? "",
  dataflow: (step) => ({
    templates: [step.url ?? "", ...(step.checks ?? []).map((c) => c.value ?? "")],
    sets: Object.keys(step.set ?? {}),
    problems: [...(step.url?.trim() ? [] : ["No URL"]), ...setProblems(step.set, SOURCES)],
  }),
  Editor: (props) => (
    <Fields>
      <TextInput label="Host or HTTPS URL to check" mono placeholder="https://" {...bind(props, "url")} />
      <NumberInput
        label="Report a certificate expiring within this many days"
        placeholder={String(DEFAULT_WARN_DAYS)}
        {...bind(props, "warn_days")}
      />
      <FailOnInput rules={RULES} {...bind(props, "fail_on")} />
      <ChecksInput
        label="Also fail unless these metrics pass (e.g. days_left gt 14)"
        keyPlaceholder="days_left"
        suggest={METRICS}
        {...bind(props, "checks")}
      />
      <SetInput sources={SOURCES} {...bind(props, "set")} />
    </Fields>
  ),
});
