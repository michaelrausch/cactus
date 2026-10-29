import { ChevronDown, ChevronUp, Search } from "lucide-react";
import { MIN_SEARCH_LENGTH, matchCountLabel } from "@/domain/search";

interface BodySearchProps {
  query: string;
  onQueryChange: (query: string) => void;
  current: number;
  matchCount: number;
  onStep: (delta: number) => void;
}

export function BodySearch({ query, onQueryChange, current, matchCount, onStep }: BodySearchProps) {
  return (
    <div className="flex h-7 flex-1 items-center gap-1.5 rounded-md border bg-background/40 px-2 focus-within:border-emerald-400/40">
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
        className="min-w-0 flex-1 bg-transparent text-xs outline-none placeholder:text-muted-foreground"
      />
      {query.length >= MIN_SEARCH_LENGTH && (
        <>
          <span className="text-[11px] text-muted-foreground tabular-nums">{matchCountLabel(current, matchCount)}</span>
          <button
            type="button"
            title="Previous match"
            onClick={() => onStep(-1)}
            className="text-muted-foreground hover:text-foreground"
          >
            <ChevronUp className="size-3.5" />
          </button>
          <button
            type="button"
            title="Next match"
            onClick={() => onStep(1)}
            className="text-muted-foreground hover:text-foreground"
          >
            <ChevronDown className="size-3.5" />
          </button>
        </>
      )}
    </div>
  );
}
