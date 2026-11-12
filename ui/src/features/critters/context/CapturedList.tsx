import { useState } from "react";
import { toast } from "sonner";
import { ChevronRight, Copy, LogIn, X } from "lucide-react";
import { clearContext, logInAgain } from "@/actions/critters";
import { selectScenario } from "@/actions/scenario";
import type { ContextEntry, CritterSummary } from "@/api/client";
import { TimeAgo } from "@/components/TimeAgo";
import { IconButton } from "@/components/IconButton";
import { Button } from "@/components/ui/button";
import { decodeJwt, expiryText, isExpired, type DecodedJwt } from "@/domain/jwt";
import { useNow } from "@/hooks/useNow";
import { cn } from "@/lib/utils";
import { useCritterStore } from "@/stores/critterStore";

/** How often expiry text ("expires in 4m") updates. */
const TICK_MS = 30_000;

/** What an entry hand-edited in the layer editor says it came from. */
const EDITED = "edited";

const copy = (name: string, value: string) =>
  navigator.clipboard.writeText(value).then(
    () => toast.success(`Copied ${name}`),
    () => toast.warning("Couldn't copy to the clipboard"),
  );

function Claims({ jwt }: { jwt: DecodedJwt }) {
  return (
    <pre className="mt-2 max-h-48 overflow-auto rounded-md bg-muted/60 p-2 font-mono text-xs">
      {JSON.stringify(jwt.claims, null, 2)}
    </pre>
  );
}

interface RowProps {
  critter: CritterSummary;
  name: string;
  entry: ContextEntry;
  now: number;
}

function CapturedRow({ critter, name, entry, now }: RowProps) {
  const [revealed, setRevealed] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const jwt = decodeJwt(entry.value);
  const expired = isExpired(jwt?.expiresAt ?? null, now);
  return (
    <li className="flex flex-col border-b py-2 last:border-b-0">
      <div className="flex items-center gap-2 text-sm">
        {jwt ? (
          <button
            type="button"
            aria-expanded={expanded}
            aria-label={expanded ? `Hide ${name}'s claims` : `Show ${name}'s claims`}
            onClick={() => setExpanded(!expanded)}
            className="rounded focus-ring"
          >
            <ChevronRight className={cn("size-3.5 transition-transform", expanded && "rotate-90")} />
          </button>
        ) : (
          <span className="w-3.5" />
        )}
        <span className="w-40 shrink-0 truncate font-mono text-captured">{name}</span>
        <button
          type="button"
          onClick={() => setRevealed(!revealed)}
          title={revealed ? "Hide value" : "Show value"}
          className="min-w-0 flex-1 truncate rounded text-left font-mono text-xs text-muted-foreground focus-ring"
        >
          {revealed ? entry.value : "••••••••"}
        </button>
        {jwt && jwt.expiresAt !== null && (
          <span className={cn("shrink-0 text-xs", expired ? "text-danger" : "text-muted-foreground")}>
            {expiryText(jwt.expiresAt, now)}
          </span>
        )}
        {expired && critter.login && (
          <Button size="sm" variant="outline" className="h-6" onClick={() => void logInAgain(critter.id)}>
            <LogIn /> Log in again
          </Button>
        )}
        {entry.from === EDITED ? (
          <span className="shrink-0 text-xs text-muted-foreground">edited</span>
        ) : (
          <button
            type="button"
            title={`Captured by ${entry.from}; open it`}
            onClick={() => void selectScenario(entry.from)}
            className="max-w-40 shrink-0 truncate rounded text-xs text-muted-foreground underline-offset-2 focus-ring hover:text-foreground hover:underline"
          >
            {entry.from}
          </button>
        )}
        <span className="w-8 shrink-0 text-right text-xs text-muted-foreground">
          <TimeAgo at={entry.at} />
        </span>
        <IconButton label={`Copy ${name}`} onClick={() => void copy(name, entry.value)}>
          <Copy />
        </IconButton>
        <IconButton label={`Forget ${name}`} onClick={() => void clearContext(critter.id, [name])}>
          <X />
        </IconButton>
      </div>
      {expanded && jwt && <Claims jwt={jwt} />}
    </li>
  );
}

/** What the critter's requests captured, kept apart from every other critter's. */
export function CapturedList({ critter }: { critter: CritterSummary }) {
  const context = useCritterStore((s) => s.contexts[critter.id]);
  const now = useNow(TICK_MS);
  const names = Object.keys(context ?? {}).toSorted();
  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <h3 className="text-sm font-semibold">Captured</h3>
        <span className="font-mono text-xs text-muted-foreground">.cactus/critters/{critter.id}/context.json</span>
        <Button
          variant="ghost"
          size="sm"
          className="ml-auto"
          disabled={names.length === 0}
          onClick={() => void clearContext(critter.id, null)}
        >
          Clear all
        </Button>
      </div>
      {names.length > 0 && (
        <ul>
          {names.map((name) => {
            const entry = context?.[name];
            return entry && <CapturedRow key={name} critter={critter} name={name} entry={entry} now={now} />;
          })}
        </ul>
      )}
    </section>
  );
}
