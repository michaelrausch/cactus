import { memo, useState } from "react";
import { ChevronRight } from "lucide-react";
import type { JsonValue } from "@/api/client";
import { cn } from "@/lib/utils";

/** Children rendered per "show more" click, so huge arrays stay responsive. */
const CHILDREN_PAGE_SIZE = 200;
/** Levels expanded on first render. */
const INITIALLY_OPEN_DEPTH = 2;

function Scalar({ value }: { value: JsonValue }) {
  if (value === null) return <span className="text-muted-foreground">null</span>;
  if (typeof value === "string") return <span className="break-all text-syntax-string">"{value}"</span>;
  if (typeof value === "number") return <span className="text-syntax-number">{value}</span>;
  return <span className="text-syntax-literal">{String(value)}</span>;
}

function Label({ name }: { name: string | undefined }) {
  if (name === undefined) return null;
  return (
    <>
      <span className="text-syntax-key">{name}</span>
      <span className="text-muted-foreground">: </span>
    </>
  );
}

type Container = JsonValue[] | { [key: string]: JsonValue };

/** The first `limit` children, without building entries for the rest. */
function childrenOf(value: Container, limit: number): [string, JsonValue][] {
  if (Array.isArray(value)) return value.slice(0, limit).map((child, index) => [String(index), child]);
  const entries: [string, JsonValue][] = [];
  for (const key in value) {
    if (entries.length >= limit) break;
    if (Object.hasOwn(value, key)) entries.push([key, value[key] as JsonValue]);
  }
  return entries;
}

const childCount = (value: Container): number => (Array.isArray(value) ? value.length : Object.keys(value).length);

/** Memoised: values never change in place, so a node only re-renders when it is toggled. */
const Node = memo(function Node({ name, value, depth }: { name?: string; value: JsonValue; depth: number }) {
  const [open, setOpen] = useState(depth < INITIALLY_OPEN_DEPTH);
  const [shown, setShown] = useState(CHILDREN_PAGE_SIZE);

  if (value === null || typeof value !== "object") {
    return (
      <div className="pl-4">
        <Label name={name} />
        <Scalar value={value} />
      </div>
    );
  }

  const isArray = Array.isArray(value);
  const count = childCount(value);
  const [openBracket, closeBracket] = isArray ? ["[", "]"] : ["{", "}"];
  const hidden = count - shown;

  return (
    <div className={cn(depth > 0 && "pl-4")}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="-ml-4 inline-flex items-center hover:text-foreground"
      >
        <ChevronRight className={cn("size-3.5 text-muted-foreground transition-transform", open && "rotate-90")} />
        <Label name={name} />
        <span className="text-muted-foreground">{openBracket}</span>
        {!open && (
          <span className="text-muted-foreground">
            {" "}
            {count} {isArray ? "items" : "keys"} {closeBracket}
          </span>
        )}
      </button>
      {open && (
        <>
          <div className="border-l border-border/60">
            {childrenOf(value, shown).map(([key, child]) => (
              <Node key={key} name={key} value={child} depth={depth + 1} />
            ))}
            {hidden > 0 && (
              <button
                type="button"
                className="pl-4 text-xs text-syntax-string hover:underline"
                onClick={() => setShown(shown + CHILDREN_PAGE_SIZE)}
              >
                show {Math.min(CHILDREN_PAGE_SIZE, hidden)} more of {hidden}
              </button>
            )}
          </div>
          <span className="text-muted-foreground">{closeBracket}</span>
        </>
      )}
    </div>
  );
});

export function JsonTree({ value }: { value: JsonValue }) {
  return (
    <div className="pl-4 font-mono text-xs leading-relaxed">
      <Node value={value} depth={0} />
    </div>
  );
}
