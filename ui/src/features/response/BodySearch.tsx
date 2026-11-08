import { ChevronDown, ChevronUp, Search } from "lucide-react";
import { MIN_SEARCH_LENGTH, matchCountLabel } from "@/domain/search";

const STEP_BUTTON =
  "focus-ring grid size-6 place-items-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground";

interface BodySearchProps {
  query: string;
  onQueryChange: (query: string) => void;
  current: number;
  matchCount: number;
  onStep: (delta: number) => void;
}

export function BodySearch({ query, onQueryChange, current, matchCount, onStep }: BodySearchProps) {
  return (
    <div className="flex h-7 flex-1 items-center gap-1.5 rounded-md border bg-background/40 px-2 focus-within:border-ring">
      <Search className="size-3.5 text-muted-foreground" />
      <input
        value={query}
        onChange={(e) => onQueryChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key !== "Enter") return;
          e.preventDefault();
          onStep(e.shiftKey ? -1 : 1);
        }}
        placeholder="Search body"
        aria-label="Search body"
        className="min-w-0 flex-1 bg-transparent text-xs outline-none placeholder:text-muted-foreground"
      />
      {query.length >= MIN_SEARCH_LENGTH && (
        <>
          <span aria-live="polite" className="text-xs text-muted-foreground tabular-nums">
            {matchCountLabel(current, matchCount)}
          </span>
          <button
            type="button"
            title="Previous match (Shift+Enter)"
            aria-label="Previous match"
            onClick={() => onStep(-1)}
            className={STEP_BUTTON}
          >
            <ChevronUp className="size-3.5" />
          </button>
          <button
            type="button"
            title="Next match (Enter)"
            aria-label="Next match"
            onClick={() => onStep(1)}
            className={STEP_BUTTON}
          >
            <ChevronDown className="size-3.5" />
          </button>
        </>
      )}
    </div>
  );
}
