import { useMemo } from "react";
import { Field } from "@/components/Field";
import { KeyValueTable } from "@/components/KeyValueTable";
import { makeRow, recordFromRows } from "@/domain/rows";
import { pathParams, queryRows, withPathParamValues, withQueryRows } from "@/domain/scenario";
import type { ScenarioTabProps } from "../types";

export function ParamsTab({ scenario, onChange }: ScenarioTabProps) {
  const query = useMemo(() => queryRows(scenario), [scenario]);
  const path = useMemo(() => pathParams(scenario).map((param) => makeRow(param.name, param.value)), [scenario]);
  return (
    <div className="flex flex-col gap-4">
      <Field label="Query">
        <KeyValueTable
          rows={query}
          toggles
          valueColumn={{ templates: true }}
          onChange={(rows) => onChange(withQueryRows(scenario, rows))}
        />
      </Field>
      {path.length > 0 && (
        <Field label="Path">
          <KeyValueTable
            rows={path}
            fixed
            valueColumn={{ templates: true }}
            onChange={(rows) => onChange(withPathParamValues(scenario, recordFromRows(rows)))}
          />
        </Field>
      )}
    </div>
  );
}
