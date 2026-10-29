import { useState } from "react";
import { X } from "lucide-react";
import { Input } from "@/components/ui/input";
import { filledRows, makeRow, rowsFingerprint, type Row } from "@/domain/rows";
import { cn } from "@/lib/utils";

interface KeyValueTableProps {
  rows: Row[];
  /** Receives only filled rows. */
  onChange: (rows: Row[]) => void;
  /** Show an enabled checkbox per row. */
  toggles?: boolean;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
}

const withTrailingBlank = (rows: Row[]): Row[] => [...rows, makeRow()];

const CELL_CLASS =
  "h-9 rounded-none border-0 bg-transparent font-mono text-xs shadow-none focus-visible:ring-0 dark:bg-transparent";

/**
 * Editable key/value list with a trailing blank row. It keeps its own rows,
 * so half-typed rows survive, and resets only when `rows` changes from outside.
 */
export function KeyValueTable({
  rows,
  onChange,
  toggles = false,
  keyPlaceholder = "Key",
  valuePlaceholder = "Value",
}: KeyValueTableProps) {
  const [local, setLocal] = useState<Row[]>(() => withTrailingBlank(rows));
  /** Fingerprint of the rows last seen from props or sent to `onChange`. */
  const [knownFingerprint, setKnownFingerprint] = useState(() => rowsFingerprint(rows));

  const incoming = rowsFingerprint(rows);
  if (incoming !== knownFingerprint) {
    setKnownFingerprint(incoming);
    setLocal(withTrailingBlank(rows));
  }

  const commit = (next: Row[]) => {
    const last = next.at(-1);
    const complete = !last || last.key || last.value ? withTrailingBlank(next) : next;
    setLocal(complete);
    setKnownFingerprint(rowsFingerprint(complete));
    onChange(filledRows(complete));
  };
  const updateRow = (id: number, patch: Partial<Row>) =>
    commit(local.map((row) => (row.id === id ? { ...row, ...patch } : row)));
  const removeRow = (id: number) => commit(local.filter((row) => row.id !== id));

  return (
    <div className="overflow-hidden rounded-lg border">
      {local.map((row, index) => {
        const isBlank = index === local.length - 1;
        return (
          <div
            key={row.id}
            className={cn("group flex items-center border-b last:border-b-0", !row.enabled && "opacity-45")}
          >
            {toggles && (
              <label className="grid w-8 shrink-0 place-items-center self-stretch">
                <input
                  type="checkbox"
                  className="size-3.5 accent-emerald-400"
                  checked={row.enabled}
                  disabled={isBlank}
                  onChange={(e) => updateRow(row.id, { enabled: e.target.checked })}
                />
              </label>
            )}
            <Input
              className={CELL_CLASS}
              placeholder={keyPlaceholder}
              value={row.key}
              spellCheck={false}
              onChange={(e) => updateRow(row.id, { key: e.target.value })}
            />
            <div className="h-5 w-px shrink-0 bg-border" />
            <Input
              className={CELL_CLASS}
              placeholder={valuePlaceholder}
              value={row.value}
              spellCheck={false}
              onChange={(e) => updateRow(row.id, { value: e.target.value })}
            />
            <button
              type="button"
              className="invisible grid w-8 shrink-0 place-items-center self-stretch text-muted-foreground group-hover:visible hover:text-foreground disabled:invisible"
              disabled={isBlank}
              onClick={() => removeRow(row.id)}
              title="Remove"
            >
              <X className="size-3.5" />
            </button>
          </div>
        );
      })}
    </div>
  );
}
