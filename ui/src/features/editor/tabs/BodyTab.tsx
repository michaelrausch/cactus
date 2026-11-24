import { useMemo } from "react";
import { KeyValueTable, type RowKinds } from "@/components/KeyValueTable";
import { TemplateField } from "@/components/template/TemplateField";
import { BODY_KIND_LABELS, BODY_KINDS, type BodyKind } from "@/domain/http";
import { multipartFromRows, multipartRows } from "@/domain/multipart";
import { listFromRows, rowsFromList, type Row } from "@/domain/rows";
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
/** A multipart part is text, or a file whose value is a path relative to the project. */
const PART_KINDS: RowKinds = {
  options: [
    { value: "text", label: "Text" },
    { value: "file", label: "File" },
  ],
  fallback: "text",
};

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
        valuePlaceholder={multipart ? "Value, or a file path" : "Value"}
        valueColumn={{ templates: true }}
        {...(multipart ? { kinds: PART_KINDS } : {})}
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
