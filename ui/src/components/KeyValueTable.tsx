import { useState } from "react";
import { X } from "lucide-react";
import { TemplateField } from "@/components/template/TemplateField";
import { Input } from "@/components/ui/input";
import { filledRows, makeRow, rowsFingerprint, type Row } from "@/domain/rows";
import type { Suggester } from "@/domain/suggestions";
import { cn } from "@/lib/utils";

/** How a column edits its text; plain unless it takes placeholders or completions. */
export interface ColumnOptions {
  /** Colour and complete `{{name}}` placeholders. */
  templates?: boolean;
  suggest?: Suggester;
  /** Draw the text as dots, e.g. for secrets. */
  masked?: boolean;
}

interface KeyValueTableProps {
  rows: Row[];
  /** Receives only filled rows. */
  onChange: (rows: Row[]) => void;
  /** Show an enabled checkbox per row. */
  toggles?: boolean;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
  /**
   * The rows as they will come back in `rows` after `onChange`, for parents
   * that normalise what they store (`eq 200 ` saved and shown as `eq 200`).
   * Lets the table recognise its own edit instead of resetting the text being typed.
   */
  echo?: (rows: Row[]) => Row[];
  keyColumn?: ColumnOptions;
  valueColumn?: ColumnOptions;
  /** A per-row choice shown before the value, e.g. text or file; rows without one show `fallback`. */
  kinds?: RowKinds;
  /** Exactly these rows: their keys can't be edited, and rows can't be added or removed. */
  fixed?: boolean;
}

export interface RowKinds {
  options: readonly { value: string; label: string }[];
  fallback: string;
}

const unchanged = (rows: Row[]): Row[] => rows;

const withTrailingBlank = (rows: Row[]): Row[] => [...rows, makeRow()];

const CELL_CLASS =
  "h-9 rounded-none border-0 bg-transparent font-mono text-xs shadow-none focus-visible:ring-0 dark:bg-transparent";
/** The text metrics of `Input` with `CELL_CLASS` (its `md:text-sm` included), for cells drawn by `TemplateField`. */
const TEMPLATE_CELL_CLASS = "h-9 px-2.5 py-1 font-mono text-xs md:text-sm";

interface CellProps {
  value: string;
  placeholder: string;
  options: ColumnOptions | undefined;
  onChange: (value: string) => void;
  readOnly?: boolean;
}

function Cell({ value, placeholder, options, onChange, readOnly = false }: CellProps) {
  if (!readOnly && !options?.masked && (options?.templates || options?.suggest)) {
    return (
      <TemplateField
        // Sized like `Input` (`w-full`) so both columns share the row evenly.
        className="w-full"
        fieldClassName={TEMPLATE_CELL_CLASS}
        value={value}
        placeholder={placeholder}
        aria-label={placeholder}
        templates={options.templates ?? false}
        suggest={options.suggest}
        onChange={onChange}
      />
    );
  }
  return (
    <Input
      type={options?.masked ? "password" : "text"}
      className={CELL_CLASS}
      placeholder={placeholder}
      aria-label={placeholder}
      value={value}
      spellCheck={false}
      readOnly={readOnly}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}

/** Cycles a row through `kinds` on click. */
function KindButton({ row, kinds, onChange }: { row: Row; kinds: RowKinds; onChange: (kind: string) => void }) {
  const current = row.kind ?? kinds.fallback;
  const index = kinds.options.findIndex((option) => option.value === current);
  const next = kinds.options[(index + 1) % kinds.options.length]?.value ?? kinds.fallback;
  const label = kinds.options[index]?.label ?? current;
  return (
    <button
      type="button"
      title={`${label}: click to change`}
      aria-label={`${row.key || "Row"} is ${label}; change`}
      onClick={() => onChange(next)}
      className="h-6 w-12 shrink-0 rounded-md text-[11px] text-muted-foreground focus-ring hover:bg-accent hover:text-foreground"
    >
      {label}
    </button>
  );
}

/**
 * Editable key/value list with a trailing blank row (unless `fixed`). It
 * keeps its own rows, so half-typed rows survive, and resets only when
 * `rows` changes from outside.
 */
export function KeyValueTable({
  rows,
  onChange,
  toggles = false,
  keyPlaceholder = "Key",
  valuePlaceholder = "Value",
  echo = unchanged,
  keyColumn,
  valueColumn,
  kinds,
  fixed = false,
}: KeyValueTableProps) {
  const withBlank = fixed ? unchanged : withTrailingBlank;
  const [local, setLocal] = useState<Row[]>(() => withBlank(rows));
  /** Fingerprint of the rows last seen from props or sent to `onChange`. */
  const [knownFingerprint, setKnownFingerprint] = useState(() => rowsFingerprint(rows));

  const incoming = rowsFingerprint(rows);
  if (incoming !== knownFingerprint) {
    setKnownFingerprint(incoming);
    setLocal(withBlank(rows));
  }

  const commit = (next: Row[]) => {
    const last = next.at(-1);
    const complete = !fixed && (!last || last.key || last.value) ? withTrailingBlank(next) : next;
    const filled = filledRows(complete);
    setLocal(complete);
    setKnownFingerprint(rowsFingerprint(echo(filled)));
    onChange(filled);
  };
  const updateRow = (id: number, patch: Partial<Row>) =>
    commit(local.map((row) => (row.id === id ? { ...row, ...patch } : row)));
  const removeRow = (id: number) => commit(local.filter((row) => row.id !== id));

  return (
    <div className="overflow-hidden rounded-lg border">
      {local.map((row, index) => {
        const isBlank = !fixed && index === local.length - 1;
        return (
          <div
            key={row.id}
            className={cn("group flex items-center border-b last:border-b-0", !row.enabled && "opacity-60")}
          >
            {toggles && (
              <label className="grid w-8 shrink-0 place-items-center self-stretch">
                <input
                  type="checkbox"
                  aria-label={row.key ? `Enable ${row.key}` : "Enable row"}
                  className="size-3.5 accent-brand-2"
                  checked={row.enabled}
                  disabled={isBlank}
                  onChange={(e) => updateRow(row.id, { enabled: e.target.checked })}
                />
              </label>
            )}
            <Cell
              value={row.key}
              placeholder={keyPlaceholder}
              options={keyColumn}
              readOnly={fixed}
              onChange={(key) => updateRow(row.id, { key })}
            />
            <div className="h-5 w-px shrink-0 bg-border" />
            {kinds && <KindButton row={row} kinds={kinds} onChange={(kind) => updateRow(row.id, { kind })} />}
            <Cell
              value={row.value}
              placeholder={valuePlaceholder}
              options={valueColumn}
              onChange={(value) => updateRow(row.id, { value })}
            />
            {!fixed && (
              <button
                type="button"
                // Faded rather than hidden, so it stays in the tab order.
                className="grid w-8 shrink-0 place-items-center self-stretch text-muted-foreground opacity-0 focus-ring group-hover:opacity-100 hover:text-foreground focus-visible:opacity-100 disabled:invisible"
                disabled={isBlank}
                onClick={() => removeRow(row.id)}
                title="Remove"
                aria-label={row.key ? `Remove ${row.key}` : "Remove row"}
              >
                <X className="size-3.5" />
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
