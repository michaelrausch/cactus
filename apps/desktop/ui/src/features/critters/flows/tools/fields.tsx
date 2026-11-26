/** The building blocks of tool editors: a label and an input bound to one field of the step. */
import { useMemo, type ReactNode } from "react";
import type { Assertion, FailOn, Severity } from "@/api/client";
import { Field } from "@/components/Field";
import { KeyValueTable, type ColumnOptions } from "@/components/KeyValueTable";
import { TemplateField } from "@/components/template/TemplateField";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { assertionsFromRows, rowsFromAssertions } from "@/domain/assertions";
import { setField } from "@/domain/flowEdit";
import { failsAt } from "@/domain/findings";
import { filledRows, recordFromRows, rowsFromRecord } from "@/domain/rows";
import { operatorSuggester, type Suggester, wordSuggester } from "@/domain/suggestions";
import { cn } from "@/lib/utils";
import { useProjectStore } from "@/stores/projectStore";
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
  disabled,
}: {
  label: string;
  value: boolean | undefined;
  onChange: (value: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <label className={cn("flex items-center gap-2 text-sm", disabled && "opacity-60")}>
      <input
        type="checkbox"
        className="size-3.5 accent-brand-2"
        disabled={disabled}
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

/** One of the project's requests. */
export function RequestInput({
  label = "Request",
  value,
  onChange,
}: {
  label?: string;
  value: string | undefined;
  onChange: (value: string) => void;
}) {
  const entries = useProjectStore((s) => s.entries);
  const requests = entries.map((entry) => ({ value: entry.relPath, label: entry.title }));
  return <ChoiceInput label={label} none="Choose…" options={requests} value={value} onChange={onChange} />;
}

/** The variables a security tool's request is sent with: its step's `with`. */
export function RequestVariablesInput(props: {
  value: Record<string, string> | undefined;
  onChange: (value: Record<string, string>) => void;
}) {
  return <MapInput label="Variables to send the request with" keyPlaceholder="Variable" {...props} />;
}

/** Mirrors `FailOn` in Rust, leaving out the default, `high`. */
const FAIL_ON: readonly { value: FailOn; label: string }[] = [
  { value: "critical", label: "Critical findings only" },
  { value: "medium", label: "Medium, high or critical findings" },
  { value: "low", label: "Low findings and above" },
  { value: "info", label: "Any finding, even info" },
  { value: "never", label: "Never: report findings only" },
];

/** One check a security tool makes, as its editor lists it. */
export interface Rule {
  label: string;
  severity: Severity;
}

/** Which findings fail a security tool's step; with `rules`, each check and whether it fails at this setting. */
export function FailOnInput({
  value,
  onChange,
  rules,
}: {
  value: FailOn | undefined;
  onChange: (value: FailOn) => void;
  rules?: readonly Rule[];
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <ChoiceInput
        label="Fail the step on"
        none="High or critical findings"
        options={FAIL_ON}
        value={value}
        onChange={(next) => onChange(next as FailOn)}
      />
      {rules && (
        <ul aria-label="Checks" className="flex flex-col rounded-md border text-xs">
          {rules.map((rule) => {
            const fails = failsAt(rule.severity, value);
            return (
              <li key={rule.label} className="flex items-center gap-2 border-t px-2 py-1 first:border-t-0">
                <span className="min-w-0 flex-1">{rule.label}</span>
                <span className="w-14 text-muted-foreground">{rule.severity}</span>
                <span className={cn("w-24 text-right", fails ? "text-danger" : "text-muted-foreground")}>
                  {fails ? "Fails the step" : "Reported only"}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

/** Some of a fixed set of options, at least one; the defaults are written as nothing. */
export function ChecklistInput<T extends string>({
  label,
  value,
  onChange,
  options,
  defaults,
}: {
  label: string;
  value: T[] | undefined;
  onChange: (value: T[]) => void;
  options: readonly { value: T; label: string }[];
  defaults: readonly T[];
}) {
  const chosen = value?.length ? value : defaults;
  const toggle = (option: T, on: boolean) => {
    const next = options.map((o) => o.value).filter((v) => (v === option ? on : chosen.includes(v)));
    // The defaults are written as nothing.
    const same = next.length === defaults.length && next.every((v) => defaults.includes(v));
    onChange(same ? [] : next);
  };
  return (
    <Field label={label}>
      <div className="flex flex-col gap-1.5">
        {options.map((option) => (
          <Checkbox
            key={option.value}
            label={option.label}
            value={chosen.includes(option.value)}
            onChange={(on) => toggle(option.value, on)}
            // Unticking the last would mean the defaults, ticking them all again.
            disabled={chosen.length === 1 && chosen[0] === option.value}
          />
        ))}
      </div>
    </Field>
  );
}

/** A security tool's `set`: variable → one of `sources`. */
export function SetInput({
  value,
  onChange,
  sources,
}: {
  value: Record<string, string> | undefined;
  onChange: (value: Record<string, string>) => void;
  sources: readonly string[];
}) {
  const suggest = useMemo(() => wordSuggester(sources), [sources]);
  return (
    <MapInput
      label="Save results as variables for later steps"
      keyPlaceholder="Variable"
      valuePlaceholder={sources[0] ?? "Value"}
      suggest={suggest}
      value={value}
      onChange={onChange}
    />
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
