import { ChevronRight } from "lucide-react";
import { useState } from "react";
import type { Finding, Metric, Severity } from "@/api/client";
import { countBySeverity, findingTitle, formatMetric, severityTone } from "@/domain/findings";
import { cn } from "@/lib/utils";

const DOT = { danger: "bg-danger", warning: "bg-warning", info: "bg-info", muted: "bg-muted-foreground" } as const;
const TEXT = {
  danger: "text-danger",
  warning: "text-warning",
  info: "text-info",
  muted: "text-muted-foreground",
} as const;

function SeverityDot({ severity }: { severity: Severity | undefined }) {
  return (
    <span className={cn("size-2 shrink-0 rounded-full", DOT[severityTone(severity)])} aria-label={severity ?? "info"} />
  );
}

/** `2 high · 1 medium`, each count in its severity's colour. */
export function SeverityCounts({ findings, className }: { findings: readonly Finding[]; className?: string }) {
  const counts = countBySeverity(findings);
  if (counts.length === 0) return <span className={cn("text-success", className)}>None found</span>;
  return (
    <span className={cn("flex items-center gap-2 tabular-nums", className)}>
      {counts.map(([severity, count]) => (
        <span key={severity} className={TEXT[severityTone(severity)]}>
          {count} {severity}
        </span>
      ))}
    </span>
  );
}

function FindingRow({ finding }: { finding: Finding }) {
  const [open, setOpen] = useState(false);
  const more = Boolean(finding.detail || finding.evidence);
  return (
    <li className="flex flex-col">
      <button
        type="button"
        disabled={!more}
        aria-expanded={more ? open : undefined}
        onClick={() => setOpen(!open)}
        className="flex w-full items-center gap-2 rounded-md px-1.5 py-1 text-left focus-ring enabled:hover:bg-accent/50"
      >
        <ChevronRight
          className={cn("size-3 shrink-0 text-muted-foreground", open && "rotate-90", !more && "invisible")}
        />
        <SeverityDot severity={finding.severity} />
        <span className="min-w-0 flex-1 truncate" title={findingTitle(finding)}>
          {findingTitle(finding)}
        </span>
        {finding.location && (
          <span className="max-w-[45%] truncate font-mono text-muted-foreground" title={finding.location}>
            {finding.location}
          </span>
        )}
      </button>
      {open && (
        <div className="flex flex-col gap-1.5 py-1 pr-1.5 pl-9 select-text">
          {finding.rule && <p className="font-mono text-muted-foreground">{finding.rule}</p>}
          {finding.detail && <p className="break-words whitespace-pre-wrap">{finding.detail}</p>}
          {finding.evidence && (
            <pre className="max-h-48 overflow-auto rounded-md border bg-muted/40 p-2 font-mono whitespace-pre-wrap">
              {finding.evidence}
            </pre>
          )}
        </div>
      )}
    </li>
  );
}

/** Every finding, as the runner sorted them (most severe first); click one for its detail and evidence. */
export function FindingsList({ findings, limit }: { findings: readonly Finding[]; limit?: number | undefined }) {
  const shown = limit === undefined ? findings : findings.slice(0, limit);
  return (
    <ul aria-label="Findings" className="flex flex-col text-xs">
      {shown.map((finding, index) => (
        // Two findings may share a rule and location.
        // oxlint-disable-next-line react/no-array-index-key
        <FindingRow key={`${finding.rule}:${finding.location}:${index}`} finding={finding} />
      ))}
      {shown.length < findings.length && (
        <li className="px-1.5 py-1 text-muted-foreground">+{findings.length - shown.length}</li>
      )}
    </ul>
  );
}

/** A tool's numbers, as label/value tiles. */
export function MetricStats({ metrics }: { metrics: readonly Metric[] }) {
  return (
    <dl className="flex flex-wrap gap-2 text-xs">
      {metrics.map((metric) => (
        <div key={metric.name} className="flex min-w-20 flex-col rounded-md border px-2 py-1">
          <dt className="text-muted-foreground">{metric.label || metric.name}</dt>
          <dd className="font-semibold tabular-nums">{formatMetric(metric)}</dd>
        </div>
      ))}
    </dl>
  );
}
