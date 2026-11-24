import { Clock } from "lucide-react";
import type { HttpResponse } from "@/api/client";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { StatusCode } from "@/components/StatusCode";
import { TimeAgo } from "@/components/TimeAgo";
import { formatBytes, formatDuration } from "@/lib/format";
import { useCritterStore } from "@/stores/critterStore";

/** "as Bob · logged in again": who the response was sent as, if anyone. */
function SentAs({ response }: { response: HttpResponse }) {
  const critter = useCritterStore((s) => s.critters.find((c) => c.id === response.critter));
  if (!response.critter) return null;
  const name = critter?.name ?? response.critter;
  return (
    <span
      className="flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-muted-foreground"
      title={response.relogged ? `${name} got a 401, logged in again and sent the request a second time` : undefined}
    >
      {critter && <CritterSprite species={critter.species} size={16} label={name} seed={critter.id} />}
      as {name}
      {response.relogged && <span className="text-info">· logged in again</span>}
    </span>
  );
}

/** Status, timing and size line, plus who it was sent as and warning, unresolved and failed-capture chips. */
export function ResponseSummary({ response, historyAt }: { response: HttpResponse; historyAt: number | null }) {
  return (
    <>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
        <StatusCode status={response.status} className="rounded-md bg-current/10 px-2 py-0.5 text-sm font-semibold">
          {response.status} {response.statusText}
        </StatusCode>
        <span className="text-muted-foreground tabular-nums">{formatDuration(response.timeMs)}</span>
        <span className="text-muted-foreground/50">·</span>
        <span className="text-muted-foreground tabular-nums">{formatBytes(response.sizeBytes)}</span>
        {response.truncated && <span className="text-warning">truncated</span>}
        <SentAs response={response} />
        {historyAt !== null && (
          <span className="ml-auto flex items-center gap-1 text-muted-foreground">
            <Clock className="size-3" /> <TimeAgo at={historyAt} suffix=" ago" />
          </span>
        )}
      </div>
      {(response.warnings.length > 0 || response.unresolved.length > 0 || response.captureFailures.length > 0) && (
        <div className="flex flex-wrap gap-1.5 text-xs">
          {response.warnings.map((warning) => (
            <span key={warning} className="rounded-full bg-warning/10 px-2 py-0.5 text-warning">
              {warning}
            </span>
          ))}
          {response.unresolved.map((name) => (
            <span
              key={name}
              className="rounded-full bg-warning/10 px-2 py-0.5 text-warning"
              title="Not defined in any env layer"
            >
              {`{{${name}}}`} unresolved
            </span>
          ))}
          {response.captureFailures.map((failure) => (
            <span
              key={failure.name}
              className="rounded-full bg-danger/10 px-2 py-0.5 text-danger"
              title={`${failure.expression}: ${failure.reason}`}
            >
              capture {failure.name} failed
            </span>
          ))}
        </div>
      )}
    </>
  );
}
