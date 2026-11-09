import type { HistoryEntry } from "@/api/client";
import { StatusCode } from "@/components/StatusCode";
import { formatDuration, timeAgo } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useResponseStore } from "@/stores/responseStore";

export function HistoryList() {
  const history = useResponseStore((s) => s.history);
  const historyAt = useResponseStore((s) => s.historyAt);
  const historyEntryPicked = useResponseStore((s) => s.historyEntryPicked);

  if (history.length === 0) return <p className="text-xs text-muted-foreground">No history yet.</p>;
  return (
    <div className="flex flex-col gap-0.5">
      {history.map((entry: HistoryEntry) => (
        <button
          type="button"
          key={entry.at}
          onClick={() => historyEntryPicked(entry)}
          className={cn(
            "flex items-center gap-3 rounded-md px-2 py-1.5 text-left text-xs hover:bg-accent/60",
            entry.at === historyAt && "bg-accent",
          )}
        >
          <StatusCode status={entry.response.status} className="w-8 font-semibold" />
          <span className="min-w-0 flex-1 truncate font-mono text-muted-foreground">{entry.response.url}</span>
          <span className="text-muted-foreground tabular-nums">{formatDuration(entry.response.timeMs)}</span>
          <span className="w-8 text-right text-muted-foreground">{timeAgo(entry.at)}</span>
        </button>
      ))}
    </div>
  );
}
