import { useMemo, useState } from "react";
import { Field, Hint } from "@/components/Field";
import { KeyValueTable } from "@/components/KeyValueTable";
import { Input } from "@/components/ui/input";
import { expectJsonFromRows, expectRowsFromJson, formatStatus, parseStatusList, withExpect } from "@/domain/expect";
import type { ScenarioTabProps } from "../types";

export function ChecksTab({ scenario, onChange }: ScenarioTabProps) {
  // Kept as typed so partial input like "200," isn't reformatted mid-edit.
  const [statusText, setStatusText] = useState(() => formatStatus(scenario.expect?.status));
  const rows = useMemo(() => expectRowsFromJson(scenario.expect?.json), [scenario.expect?.json]);

  const editStatus = (text: string) => {
    setStatusText(text);
    onChange(withExpect(scenario, { status: parseStatusList(text) }));
  };

  return (
    <>
      <Field label="Expected status">
        <Input
          className="w-48 font-mono text-xs"
          value={statusText}
          placeholder="200, 201"
          onChange={(e) => editStatus(e.target.value)}
        />
      </Field>
      <Field label="JSON body">
        <KeyValueTable
          rows={rows}
          keyPlaceholder="$.path"
          valuePlaceholder='expected JSON, e.g. true or "ok"'
          onChange={(next) => onChange(withExpect(scenario, { json: expectJsonFromRows(next) }))}
        />
      </Field>
      <Hint>
        Checks run on every send and when running a folder. Without a status check, any status under 400 passes.
      </Hint>
    </>
  );
}
