import { FileCheck } from "lucide-react";
import { exportSources, setProblems } from "@/domain/findings";
import { bind, FailOnInput, Fields, RequestInput, RequestVariablesInput, type Rule, SetInput } from "./fields";
import { defineTool } from "./types";

/** Mirrors `headers::EXPORTS` in Rust: findings only. */
const SOURCES = exportSources([], true);

/** What it checks; mirrors the checks in `headers.rs`. */
const RULES: readonly Rule[] = [
  { label: "CORS lets any site read responses, with credentials", severity: "high" },
  { label: "CORS lets any site read responses", severity: "medium" },
  { label: "No Strict-Transport-Security (HTTPS only)", severity: "medium" },
  { label: "No Content-Security-Policy on HTML", severity: "medium" },
  { label: "HTML can be framed by other sites (clickjacking)", severity: "medium" },
  { label: "A cookie without Secure (HTTPS only)", severity: "medium" },
  { label: "HSTS max-age under 180 days", severity: "low" },
  { label: "CSP allows 'unsafe-inline' or 'unsafe-eval' scripts", severity: "low" },
  { label: "No X-Content-Type-Options: nosniff", severity: "low" },
  { label: "No Referrer-Policy, or unsafe-url", severity: "low" },
  { label: "Server header shows a version", severity: "low" },
  { label: "X-Powered-By names the framework", severity: "low" },
  { label: "A cookie without HttpOnly or SameSite", severity: "low" },
  { label: "No Content-Security-Policy on an API response", severity: "info" },
];

export const headers = defineTool<"headers">({
  label: "Security headers",
  icon: FileCheck,
  create: () => ({ tool: "headers" }),
  summary: (step) => step.request ?? "",
  dataflow: (step) => ({
    templates: Object.values(step.with ?? {}),
    sets: Object.keys(step.set ?? {}),
    problems: [...(step.request?.trim() ? [] : ["No request"]), ...setProblems(step.set, SOURCES)],
  }),
  Editor: (props) => (
    <Fields>
      <RequestInput label="Request whose response to check" {...bind(props, "request")} />
      <RequestVariablesInput {...bind(props, "with")} />
      <FailOnInput rules={RULES} {...bind(props, "fail_on")} />
      <SetInput sources={SOURCES} {...bind(props, "set")} />
    </Fields>
  ),
});
