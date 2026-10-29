import { useMemo } from "react";
import { Hint } from "@/components/Field";
import { KeyValueTable } from "@/components/KeyValueTable";
import { Textarea } from "@/components/ui/textarea";
import { BODY_TYPES } from "@/domain/http";
import { recordFromRows, rowsFromRecord } from "@/domain/rows";
import { isJsonObject } from "@/domain/scenario";
import { cn } from "@/lib/utils";
import type { ScenarioTabProps } from "../types";
import type { BodyDraft } from "../useBodyDraft";

const JSON_PLACEHOLDER = '{\n  "key": "{{value}}"\n}';

function BodyTypePicker({ draft }: { draft: BodyDraft }) {
  return (
    <div className="flex items-center gap-1">
      {BODY_TYPES.map((type) => (
        <button
          type="button"
          key={type}
          onClick={() => draft.switchType(type)}
          className={cn(
            "rounded-md px-2.5 py-1 text-xs capitalize transition",
            type === draft.bodyType ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground",
          )}
        >
          {type}
        </button>
      ))}
      {draft.jsonError && <span className="ml-auto truncate text-xs text-rose-400">Invalid JSON, not saved</span>}
    </div>
  );
}

function FieldsEditor({ scenario, onChange, multipart }: ScenarioTabProps & { multipart: boolean }) {
  const rows = useMemo(
    () => (isJsonObject(scenario.body) ? rowsFromRecord(scenario.body as Record<string, string>) : []),
    [scenario.body],
  );
  return (
    <div className="overflow-auto">
      <KeyValueTable
        rows={rows}
        onChange={(next) => onChange({ ...scenario, body: recordFromRows(next) })}
        valuePlaceholder={multipart ? "Value, or @path/to/file" : "Value"}
      />
      {multipart && <Hint>Start a value with @ to upload a file. Relative paths resolve from the project folder.</Hint>}
    </div>
  );
}

export function BodyTab({ draft, ...tab }: ScenarioTabProps & { draft: BodyDraft }) {
  const { bodyType } = draft;
  return (
    <>
      <BodyTypePicker draft={draft} />
      {(bodyType === "json" || bodyType === "text") && (
        <Textarea
          value={draft.text}
          onChange={(e) => draft.editText(e.target.value)}
          spellCheck={false}
          placeholder={bodyType === "json" ? JSON_PLACEHOLDER : ""}
          className="min-h-0 flex-1 resize-none rounded-lg font-mono text-xs leading-relaxed"
        />
      )}
      {(bodyType === "form" || bodyType === "multipart") && (
        <FieldsEditor {...tab} multipart={bodyType === "multipart"} />
      )}
      {bodyType === "none" && <Hint>This request has no body.</Hint>}
    </>
  );
}
