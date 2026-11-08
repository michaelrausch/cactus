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
}

function Cell({ value, placeholder, options, onChange }: CellProps) {
  if (!options?.masked && (options?.templates || options?.suggest)) {
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
      onChange={(e) => onChange(e.target.value)}
    />
  );
}

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
  echo = unchanged,
  keyColumn,
  valueColumn,
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
        const isBlank = index === local.length - 1;
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
              onChange={(key) => updateRow(row.id, { key })}
            />
            <div className="h-5 w-px shrink-0 bg-border" />
            <Cell
              value={row.value}
              placeholder={valuePlaceholder}
              options={valueColumn}
              onChange={(value) => updateRow(row.id, { value })}
            />
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
          </div>
        );
      })}
    </div>
  );
}
