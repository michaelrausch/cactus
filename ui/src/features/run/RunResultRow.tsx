import type { RunResult } from "@/api/client";
import { openRunResult } from "@/actions/request";
import { StatusCode } from "@/components/StatusCode";
import { CheckIcon } from "@/features/response/ChecksList";

const DETAIL_CLASS = "pl-[4.25rem] text-xs text-rose-300";

export function RunResultRow({ result }: { result: RunResult }) {
  const { response } = result;
  return (
    <button
      type="button"
      onClick={() => openRunResult(result.relPath)}
      className="flex w-full flex-col gap-0.5 rounded-md px-2 py-1.5 text-left hover:bg-accent/60"
    >
      <div className="flex w-full items-center gap-3 text-sm">
        <CheckIcon ok={result.ok} className="size-4 shrink-0" />
        {response ? (
          <StatusCode status={response.status} className="w-8 text-xs" />
        ) : (
          <span className="w-8 font-mono text-xs">—</span>
        )}
        <span className="min-w-0 flex-1 truncate">{result.title}</span>
        <span className="text-xs text-muted-foreground tabular-nums">{response ? `${response.timeMs}ms` : ""}</span>
      </div>
      {result.error && <span className={DETAIL_CLASS}>{result.error}</span>}
      {response?.checks
        .filter((check) => !check.ok)
        .map((check) => (
          <span key={check.name} className={`${DETAIL_CLASS} font-mono`}>
            {check.name}: {check.detail}
          </span>
        ))}
      {response?.captureFailures.map((failure) => (
        <span key={failure.name} className={`${DETAIL_CLASS} font-mono`}>
          capture {failure.name}: {failure.reason}
        </span>
      ))}
    </button>
  );
}
