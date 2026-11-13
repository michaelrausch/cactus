import { useState } from "react";
import { UserRound } from "lucide-react";
import type { HistoryEntry } from "@/api/client";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { StatusCode } from "@/components/StatusCode";
import { TimeAgo } from "@/components/TimeAgo";
import { formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useCritterStore } from "@/stores/critterStore";
import { useResponseStore } from "@/stores/responseStore";

/** Every entry, no critter, or one critter's id. */
const ALL = "__all__";
const NONE = "__none__";

const senderOf = (entry: HistoryEntry): string => entry.response.critter ?? NONE;

/** Who sent an entry: the critter's sprite, or a plain person for nobody. */
function Sender({ id }: { id: string | null }) {
  const critter = useCritterStore((s) => s.critters.find((c) => c.id === id));
  if (!id) return <UserRound className="size-4 shrink-0 text-muted-foreground/60" aria-label="No critter" />;
  if (!critter)
    return (
      <span className="w-4 shrink-0 truncate text-muted-foreground" title={id}>
        ?
      </span>
    );
  return <CritterSprite species={critter.species} size={16} label={`as ${critter.name}`} seed={critter.id} />;
}

/** The open request's recent responses, filterable by who sent them. */
export function HistoryList() {
  const history = useResponseStore((s) => s.history);
  const historyAt = useResponseStore((s) => s.historyAt);
  const historyEntryPicked = useResponseStore((s) => s.historyEntryPicked);
  const critters = useCritterStore((s) => s.critters);
  const [filter, setFilter] = useState(ALL);

  if (history.length === 0) return <p className="text-xs text-muted-foreground">No history yet.</p>;
  const senders = [...new Set(history.map(senderOf))];
  const shown = filter === ALL ? history : history.filter((entry) => senderOf(entry) === filter);
  const nameOf = (sender: string) =>
    sender === NONE ? "No critter" : (critters.find((c) => c.id === sender)?.name ?? sender);
  return (
    <div className="flex flex-col gap-0.5">
      {senders.length > 1 && (
        <label className="mb-1 flex items-center gap-2 text-xs text-muted-foreground">
          Sent as
          <select
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            className="rounded-md border bg-background px-1.5 py-0.5 text-foreground"
          >
            <option value={ALL}>Anyone</option>
            {senders.map((sender) => (
              <option key={sender} value={sender}>
                {nameOf(sender)}
              </option>
            ))}
          </select>
        </label>
      )}
      {shown.map((entry: HistoryEntry) => (
        <button
          type="button"
          key={entry.at}
          onClick={() => historyEntryPicked(entry)}
          className={cn(
            "flex items-center gap-3 rounded-md px-2 py-1.5 text-left text-xs hover:bg-accent/60",
            entry.at === historyAt && "bg-accent",
          )}
        >
          <Sender id={entry.response.critter} />
          <StatusCode status={entry.response.status} className="w-8 font-semibold" />
          <span className="min-w-0 flex-1 truncate font-mono text-muted-foreground">{entry.response.url}</span>
          <span className="text-muted-foreground tabular-nums">{formatDuration(entry.response.timeMs)}</span>
          <span className="w-8 text-right text-muted-foreground">
            <TimeAgo at={entry.at} />
          </span>
        </button>
      ))}
    </div>
  );
}
