import { useState } from "react";
import { X } from "lucide-react";
import { removeCookies } from "@/actions/critters";
import type { CookieEntry, CritterSummary } from "@/api/client";
import { IconButton } from "@/components/IconButton";
import { Button } from "@/components/ui/button";
import { EXPIRY_TICK_MS, expiryText } from "@/domain/jwt";
import { useNow } from "@/hooks/useNow";
import { useCritterStore } from "@/stores/critterStore";

function CookieRow({ critterId, cookie, now }: { critterId: string; cookie: CookieEntry; now: number }) {
  const [revealed, setRevealed] = useState(false);
  const { name, domain, path } = cookie;
  return (
    <li className="flex items-center gap-2 border-b py-2 text-sm last:border-b-0">
      <span className="w-40 shrink-0 truncate font-mono">{name}</span>
      <button
        type="button"
        onClick={() => setRevealed(!revealed)}
        title={revealed ? "Hide value" : "Show value"}
        className="min-w-0 flex-1 truncate rounded text-left font-mono text-xs text-muted-foreground focus-ring"
      >
        {revealed ? cookie.value : "••••••••"}
      </button>
      <span className="max-w-48 shrink-0 truncate text-xs text-muted-foreground" title={`${domain}${path}`}>
        {domain}
        {path === "/" ? "" : path}
      </span>
      <span className="w-28 shrink-0 text-right text-xs text-muted-foreground">
        {cookie.expires === null ? "session" : expiryText(cookie.expires, now)}
      </span>
      <IconButton label={`Remove ${name}`} onClick={() => void removeCookies(critterId, [{ domain, path, name }])}>
        <X />
      </IconButton>
    </li>
  );
}

/** The critter's cookie jar: what servers set while sending as it. */
export function CookiesList({ critter }: { critter: CritterSummary }) {
  const cookies = useCritterStore((s) => s.cookies[critter.id]) ?? [];
  const now = useNow(EXPIRY_TICK_MS);
  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <h3 className="text-sm font-semibold">Cookies</h3>
        <Button
          variant="ghost"
          size="sm"
          className="ml-auto"
          disabled={cookies.length === 0}
          onClick={() => void removeCookies(critter.id, null)}
        >
          Clear all
        </Button>
      </div>
      {cookies.length > 0 && (
        <ul>
          {cookies.map((cookie) => (
            <CookieRow
              key={`${cookie.domain} ${cookie.path} ${cookie.name}`}
              critterId={critter.id}
              cookie={cookie}
              now={now}
            />
          ))}
        </ul>
      )}
    </section>
  );
}
