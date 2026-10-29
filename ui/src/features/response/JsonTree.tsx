import { useState } from "react";
import { ChevronRight } from "lucide-react";
import type { JsonValue } from "@/api/client";
import { cn } from "@/lib/utils";

/** Children rendered per "show more" click, so huge arrays stay responsive. */
const CHILDREN_PAGE_SIZE = 200;
/** Levels expanded on first render. */
const INITIALLY_OPEN_DEPTH = 2;

function Scalar({ value }: { value: JsonValue }) {
  if (value === null) return <span className="text-muted-foreground">null</span>;
  if (typeof value === "string") return <span className="break-all text-emerald-300">"{value}"</span>;
  if (typeof value === "number") return <span className="text-amber-300">{value}</span>;
  return <span className="text-violet-300">{String(value)}</span>;
}

function Label({ name }: { name: string | undefined }) {
  if (name === undefined) return null;
  return (
    <>
      <span className="text-sky-300">{name}</span>
      <span className="text-muted-foreground">: </span>
    </>
  );
}

const childrenOf = (value: JsonValue[] | { [key: string]: JsonValue }): [string, JsonValue][] =>
  Array.isArray(value) ? value.map((child, index) => [String(index), child]) : Object.entries(value);

function Node({ name, value, depth }: { name?: string; value: JsonValue; depth: number }) {
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
  const children = childrenOf(value);
  const [openBracket, closeBracket] = isArray ? ["[", "]"] : ["{", "}"];
  const hidden = children.length - shown;

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
            {children.length} {isArray ? "items" : "keys"} {closeBracket}
          </span>
        )}
      </button>
      {open && (
        <>
          <div className="border-l border-border/60">
            {children.slice(0, shown).map(([key, child]) => (
              <Node key={key} name={key} value={child} depth={depth + 1} />
            ))}
            {hidden > 0 && (
              <button
                type="button"
                className="pl-4 text-xs text-emerald-300 hover:underline"
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
}

export function JsonTree({ value }: { value: JsonValue }) {
  return (
    <div className="pl-4 font-mono text-xs leading-relaxed">
      <Node value={value} depth={0} />
    </div>
  );
}
