import { useMemo } from "react";
import { KeyValueTable } from "@/components/KeyValueTable";
import { assertionRows, withAssertionRows } from "@/domain/assertions";
import type { ScenarioTabProps } from "../types";

export function AssertionsTab({ scenario, onChange }: ScenarioTabProps) {
  const rows = useMemo(() => assertionRows(scenario), [scenario]);
  return (
    <>
      <KeyValueTable
        rows={rows}
        toggles
        keyPlaceholder="res.status"
        valuePlaceholder="eq 200"
        onChange={(next) => onChange(withAssertionRows(scenario, next))}
      />
    </>
  );
}
