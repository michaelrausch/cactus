import { useMemo, useState } from "react";
import type { HttpResponse } from "@/api/client";
import { displayBody, parseBody } from "@/domain/responseBody";
import { findMatches, stepMatch } from "@/domain/search";
import { cn } from "@/lib/utils";
import { BodySearch } from "./BodySearch";
import { JsonTree } from "./JsonTree";
import { SearchText } from "./SearchText";

type BodyView = "tree" | "raw";
const BODY_VIEWS: readonly BodyView[] = ["tree", "raw"];

function ViewToggle({
  view,
  searching,
  onChange,
}: {
  view: BodyView;
  searching: boolean;
  onChange: (v: BodyView) => void;
}) {
  return (
    <div role="group" aria-label="Body view" className="flex rounded-md border p-0.5 text-xs">
      {BODY_VIEWS.map((option) => {
        // Searching always shows raw text, so the tree option reads as inactive.
        const selected = view === option && !(option === "tree" && searching);
        return (
          <button
            type="button"
            key={option}
            aria-pressed={selected}
            onClick={() => onChange(option)}
            className={cn(
              "rounded px-2 py-0.5 capitalize focus-ring",
              selected ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground",
            )}
          >
            {option}
          </button>
        );
      })}
    </div>
  );
}

/** Response body as a JSON tree or searchable raw text. */
export function BodyViewer({ response }: { response: HttpResponse }) {
  const parsed = useMemo(() => parseBody(response), [response]);
  const text = useMemo(() => displayBody(response, parsed), [response, parsed]);
  const [view, setView] = useState<BodyView>("tree");
  const [query, setQuery] = useState("");
  const [current, setCurrent] = useState(0);
  const matches = useMemo(() => findMatches(text, query), [text, query]);

  const changeQuery = (next: string) => {
    setQuery(next);
    setCurrent(0);
  };
  const step = (delta: number) => setCurrent(stepMatch(current, delta, matches.length));

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <div className="flex items-center gap-2">
        <BodySearch
          query={query}
          onQueryChange={changeQuery}
          current={current}
          matchCount={matches.length}
          onStep={step}
        />
        {parsed.kind === "json" && <ViewToggle view={view} searching={!!query} onChange={setView} />}
      </div>
      <div className="min-h-0 flex-1 overflow-auto rounded-lg bg-background/50 p-3">
        {parsed.kind === "json" && view === "tree" && !query ? (
          <JsonTree value={parsed.value} />
        ) : (
          <SearchText text={text} queryLength={query.length} matches={matches} current={current} />
        )}
      </div>
    </div>
  );
}
