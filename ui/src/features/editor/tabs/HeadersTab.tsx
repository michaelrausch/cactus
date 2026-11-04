import { useMemo } from "react";
import { KeyValueTable } from "@/components/KeyValueTable";
import { listFromRows, rowsFromList } from "@/domain/rows";
import { withHeaders } from "@/domain/scenario";
import type { ScenarioTabProps } from "../types";

export function HeadersTab({ scenario, onChange }: ScenarioTabProps) {
  const rows = useMemo(() => rowsFromList(scenario.http.headers), [scenario.http.headers]);
  return (
    <>
      <KeyValueTable
        rows={rows}
        toggles
        valueColumn={{ templates: true }}
        onChange={(next) => onChange(withHeaders(scenario, listFromRows(next)))}
      />
    </>
  );
}
