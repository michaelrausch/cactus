import { useMemo } from "react";
import { Field } from "@/components/Field";
import { KeyValueTable } from "@/components/KeyValueTable";
import { parseQuery, withQuery } from "@/domain/query";
import { makeRow, recordFromRows } from "@/domain/rows";
import { pathParams, withPathParamValues, withUrl } from "@/domain/scenario";
import type { ScenarioTabProps } from "../types";

export function ParamsTab({ scenario, onChange }: ScenarioTabProps) {
  const query = useMemo(() => parseQuery(scenario.http.url), [scenario.http.url]);
  const path = useMemo(() => pathParams(scenario).map((param) => makeRow(param.name, param.value)), [scenario]);
  return (
    <>
      <KeyValueTable
        rows={query}
        valueColumn={{ templates: true }}
        onChange={(rows) => onChange(withUrl(scenario, withQuery(scenario.http.url, rows)))}
      />
      {path.length > 0 && (
        <Field label="Path">
          <KeyValueTable
            rows={path}
            keyPlaceholder="name"
            valueColumn={{ templates: true }}
            onChange={(rows) => onChange(withPathParamValues(scenario, recordFromRows(rows)))}
          />
        </Field>
      )}
    </>
  );
}
