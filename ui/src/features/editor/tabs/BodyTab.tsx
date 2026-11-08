import { useMemo } from "react";
import type { MultipartField } from "@/api/client";
import { KeyValueTable } from "@/components/KeyValueTable";
import { TemplateField } from "@/components/template/TemplateField";
import { BODY_KIND_LABELS, BODY_KINDS, type BodyKind } from "@/domain/http";
import { listFromRows, makeRow, rowsFromList, type Row } from "@/domain/rows";
import {
  bodyKind,
  bodyText,
  isTextBodyKind,
  jsonProblem,
  switchBodyKind,
  typedBody,
  withBody,
  withBodyText,
} from "@/domain/scenario";
import { cn } from "@/lib/utils";
import type { ScenarioTabProps } from "../types";

const JSON_PLACEHOLDER = '{\n  "key": "{{value}}"\n}';
/** A multipart value starting with this is a file path, relative to the project. */
const FILE_PREFIX = "@";

function BodyKindPicker({ scenario, onChange }: ScenarioTabProps) {
  const current = bodyKind(scenario);
  return (
    <div role="group" aria-label="Body type" className="flex items-center gap-1">
      {BODY_KINDS.map((kind) => (
        <button
          type="button"
          key={kind}
          aria-pressed={kind === current}
          onClick={() => onChange(switchBodyKind(scenario, kind))}
          className={cn(
            "rounded-md px-2.5 py-1 text-xs focus-ring transition",
            kind === current ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground",
          )}
        >
          {BODY_KIND_LABELS[kind]}
        </button>
      ))}
    </div>
  );
}

/** Multipart file parts are shown as `@path`, as in curl. */
const multipartRows = (fields: MultipartField[]): Row[] =>
  fields.map((field) => {
    const value = Array.isArray(field.value) ? field.value.join(", ") : field.value;
    return makeRow(field.name, field.type === "file" ? `${FILE_PREFIX}${value}` : value, !field.disabled);
  });

const multipartFromRows = (rows: Row[]): MultipartField[] =>
  rows.map((row) => {
    const isFile = row.value.startsWith(FILE_PREFIX);
    const field: MultipartField = {
      name: row.key,
      type: isFile ? "file" : "text",
      value: isFile ? row.value.slice(FILE_PREFIX.length) : row.value,
    };
    return row.enabled ? field : { ...field, disabled: true };
  });

function FieldsEditor({ scenario, onChange, kind }: ScenarioTabProps & { kind: "form-urlencoded" | "multipart-form" }) {
  const body = typedBody(scenario);
  const rows = useMemo(() => {
    if (body?.type === "form-urlencoded") return rowsFromList(body.data);
    if (body?.type === "multipart-form") return multipartRows(body.data);
    return [];
  }, [body]);
  const update = (next: Row[]) =>
    onChange(
      withBody(
        scenario,
        kind === "form-urlencoded"
          ? { type: kind, data: listFromRows(next) }
          : { type: kind, data: multipartFromRows(next) },
      ),
    );
  const multipart = kind === "multipart-form";
  return (
    <div className="overflow-auto">
      <KeyValueTable
        rows={rows}
        toggles
        onChange={update}
        valuePlaceholder={multipart ? "Value, or @path/to/file" : "Value"}
        valueColumn={{ templates: true }}
      />
    </div>
  );
}

function TextEditor({ scenario, onChange, kind }: ScenarioTabProps & { kind: "json" | "text" | "xml" }) {
  const text = bodyText(scenario);
  const problem = kind === "json" ? jsonProblem(text) : null;
  return (
    <>
      <div className="min-h-0 flex-1 overflow-auto rounded-lg border border-input bg-input/30 transition-colors focus-within:border-ring">
        <TemplateField
          multiline
          syntax={kind === "json" ? "json" : "template"}
          value={text}
          onChange={(next) => onChange(withBodyText(scenario, kind, next))}
          placeholder={kind === "json" ? JSON_PLACEHOLDER : ""}
          aria-label="Body"
          className="min-h-full"
          fieldClassName="px-2.5 py-2 font-mono text-xs leading-relaxed md:text-sm"
        />
      </div>
      {problem && <span className="truncate text-xs text-warning">Not valid JSON: {problem}</span>}
    </>
  );
}

export function BodyTab(tab: ScenarioTabProps) {
  const kind: BodyKind | "other" = bodyKind(tab.scenario);
  return (
    <>
      <BodyKindPicker {...tab} />
      {isTextBodyKind(kind) && <TextEditor {...tab} kind={kind} />}
      {(kind === "form-urlencoded" || kind === "multipart-form") && <FieldsEditor {...tab} kind={kind} />}
    </>
  );
}
