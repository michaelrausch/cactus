/** The building blocks of tool editors: a label and an input bound to one field of the step. */
import { useMemo, type ReactNode } from "react";
import type { Assertion } from "@/api/client";
import { Field } from "@/components/Field";
import { KeyValueTable, type ColumnOptions } from "@/components/KeyValueTable";
import { TemplateField } from "@/components/template/TemplateField";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { assertionsFromRows, rowsFromAssertions } from "@/domain/assertions";
import { setField } from "@/domain/flowEdit";
import { filledRows, recordFromRows, rowsFromRecord } from "@/domain/rows";
import { operatorSuggester, type Suggester } from "@/domain/suggestions";
import { cn } from "@/lib/utils";
import type { ToolEditorProps, ToolId, ToolOf } from "./types";

/**
 * `value` and `onChange` for field `key` of the step, for any input below:
 * `<TextInput label="URL" {...bind(props, "url")} />`. Clearing a field
 * leaves it out of the file.
 */
export function bind<T extends ToolId, K extends Exclude<keyof ToolOf<T>, "tool">>(
  { step, update }: ToolEditorProps<T>,
  key: K,
): { value: ToolOf<T>[K] | undefined; onChange: (value: ToolOf<T>[K]) => void } {
  return { value: step[key], onChange: (value) => update((s) => setField(s, key, value)) };
}

/** The settings of a tool, spaced like the rest of the step pane. */
export function Fields({ children }: { children: ReactNode }) {
  return <div className="flex flex-col gap-3">{children}</div>;
}

interface TextInputProps {
  label: string;
  value: string | undefined;
  onChange: (value: string) => void;
  placeholder?: string;
  multiline?: boolean;
  /** `{{name}}` highlighting and completion; on unless the field isn't templated. */
  templates?: boolean;
  mono?: boolean;
  /** Colour JSON too, e.g. a request body. */
  json?: boolean;
  suggest?: Suggester;
}

export function TextInput({
  label,
  value,
  onChange,
  placeholder,
  multiline,
  templates = true,
  mono,
  json,
  suggest,
}: TextInputProps) {
  return (
    <Field label={label}>
      <div className="rounded-md border focus-within:border-ring">
        <TemplateField
          value={value ?? ""}
          onChange={onChange}
          multiline={multiline ?? false}
          syntax={json ? "json" : "template"}
          templates={templates}
          suggest={suggest}
          placeholder={placeholder ?? ""}
          aria-label={label}
          fieldClassName={cn("px-2 text-sm", multiline ? "min-h-20 py-1.5" : "h-8", mono && "font-mono text-xs")}
        />
      </div>
    </Field>
  );
}

export function NumberInput({
  label,
  value,
  onChange,
  placeholder,
  min = 0,
  max,
}: {
  label: string;
  value: number | undefined;
  onChange: (value: number) => void;
  placeholder?: string;
  min?: number;
  max?: number;
}) {
  return (
    <Field label={label}>
      <Input
        type="number"
        className="h-8 w-28"
        min={min}
        max={max}
        placeholder={placeholder}
        value={value ?? ""}
        onChange={(event) => {
          const parsed = Number.parseInt(event.target.value, 10);
          onChange(Number.isNaN(parsed) ? 0 : parsed);
        }}
      />
    </Field>
  );
}

export function Checkbox({
  label,
  value,
  onChange,
}: {
  label: string;
  value: boolean | undefined;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-2 text-sm">
      <input
        type="checkbox"
        className="size-3.5 accent-brand-2"
        checked={value ?? false}
        onChange={(event) => onChange(event.target.checked)}
      />
      {label}
    </label>
  );
}

/** Checks like a request's (`res.body.status` · `eq ready`); rows without a left side are left out. */
export function ChecksInput({
  label,
  value,
  onChange,
  keyPlaceholder,
  templates = false,
  suggest,
}: {
  label: string;
  value: Assertion[] | undefined;
  onChange: (value: Assertion[]) => void;
  keyPlaceholder: string;
  /** The left side is a `{{template}}`, with variable completion. */
  templates?: boolean;
  /** Completions for the left side, e.g. `res.*` expressions or an item's fields. */
  suggest?: Suggester;
}) {
  const rows = useMemo(() => rowsFromAssertions(value), [value]);
  let keyColumn: ColumnOptions = {};
  if (templates) keyColumn = { templates: true };
  else if (suggest) keyColumn = { suggest };
  return (
    <Field label={label}>
      <KeyValueTable
        rows={rows}
        toggles
        keyPlaceholder={keyPlaceholder}
        valuePlaceholder="eq ok"
        keyColumn={keyColumn}
        valueColumn={{ templates: true, suggest: operatorSuggester }}
        echo={(next) => rowsFromAssertions(assertionsFromRows(filledRows(next)))}
        onChange={(next) => onChange(assertionsFromRows(filledRows(next)))}
      />
    </Field>
  );
}

/** One of a fixed set of options. `""` is a valid value, shown as `none`. */
export function ChoiceInput({
  label,
  value,
  onChange,
  options,
  none,
}: {
  label: string;
  value: string | undefined;
  onChange: (value: string) => void;
  options: readonly { value: string; label: string }[];
  /** The label of the empty value, e.g. "This critter". */
  none: string;
}) {
  // Select items can't have an empty value, so the empty one stands in.
  const NONE = "\u0000";
  const known = options.some((option) => option.value === value);
  return (
    <Field label={label}>
      <Select value={value ? value : NONE} onValueChange={(next) => onChange(next === NONE ? "" : next)}>
        <SelectTrigger className="h-8 text-sm" aria-label={label}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={NONE}>{none}</SelectItem>
          {value && !known && <SelectItem value={value}>{value}</SelectItem>}
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Field>
  );
}

/** A name → value map, e.g. headers; rows without a name are left out. */
export function MapInput({
  label,
  value,
  onChange,
  keyPlaceholder,
  valuePlaceholder,
  suggest,
}: {
  label: string;
  value: Record<string, string> | undefined;
  onChange: (value: Record<string, string>) => void;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
  suggest?: Suggester;
}) {
  const rows = useMemo(() => rowsFromRecord(value), [value]);
  return (
    <Field label={label}>
      <KeyValueTable
        rows={rows}
        keyPlaceholder={keyPlaceholder ?? "Name"}
        valuePlaceholder={valuePlaceholder ?? "Value"}
        valueColumn={suggest ? { templates: true, suggest } : { templates: true }}
        onChange={(next) => onChange(recordFromRows(filledRows(next)))}
      />
    </Field>
  );
}
