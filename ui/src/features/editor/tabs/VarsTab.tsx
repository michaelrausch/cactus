import { useMemo } from "react";
import { Hint } from "@/components/Field";
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
      <Hint>
        After each send, what the expression selects is saved as a variable (in .cactus/captured.json) so later requests
        can use it as {"{{name}}"}.
      </Hint>
    </>
  );
}
