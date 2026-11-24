import { useMemo } from "react";
import { KeyValueTable } from "@/components/KeyValueTable";
import { varRows, withVarRows } from "@/domain/assertions";
import type { ScenarioTabProps } from "../types";

export function VarsTab({ scenario, onChange }: ScenarioTabProps) {
  const rows = useMemo(() => varRows(scenario), [scenario]);
  return (
    <>
      <KeyValueTable
        rows={rows}
        toggles
        keyPlaceholder="Variable"
        valuePlaceholder="res.body.data.token  or  res.headers.location"
        onChange={(next) => onChange(withVarRows(scenario, next))}
      />
    </>
  );
}
