import { Clock } from "lucide-react";
import type { HttpResponse } from "@/api/client";
import { StatusCode } from "@/components/StatusCode";
import { formatBytes, timeAgo } from "@/lib/format";

/** Status, timing and size line, plus warning, unresolved and captured variable chips. */
export function ResponseSummary({ response, historyAt }: { response: HttpResponse; historyAt: number | null }) {
  const captured = Object.entries(response.captured);
  return (
    <>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
        <StatusCode status={response.status} className="rounded-md bg-current/10 px-2 py-0.5 text-[13px] font-semibold">
          {response.status} {response.statusText}
        </StatusCode>
        <span className="text-muted-foreground tabular-nums">{response.timeMs} ms</span>
        <span className="text-muted-foreground/50">·</span>
        <span className="text-muted-foreground tabular-nums">{formatBytes(response.sizeBytes)}</span>
        {response.truncated && <span className="text-amber-400">truncated</span>}
        {historyAt !== null && (
          <span className="ml-auto flex items-center gap-1 text-muted-foreground">
            <Clock className="size-3" /> {timeAgo(historyAt)} ago
          </span>
        )}
      </div>
      {(response.warnings.length > 0 || response.unresolved.length > 0 || captured.length > 0) && (
        <div className="flex flex-wrap gap-1.5 text-[11px]">
          {response.warnings.map((warning) => (
            <span key={warning} className="rounded-full bg-amber-400/10 px-2 py-0.5 text-amber-300">
              {warning}
            </span>
          ))}
          {response.unresolved.map((name) => (
            <span
              key={name}
              className="rounded-full bg-amber-400/10 px-2 py-0.5 text-amber-300"
              title="Not defined in any env layer"
            >
              {`{{${name}}}`} unresolved
            </span>
          ))}
          {captured.map(([name, value]) => (
            <span
              key={name}
              className="max-w-60 truncate rounded-full bg-teal-400/10 px-2 py-0.5 text-teal-300"
              title={value}
            >
              captured {name}
            </span>
          ))}
        </div>
      )}
    </>
  );
}
