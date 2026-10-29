import { useMemo } from "react";
import { Hint } from "@/components/Field";
import { KeyValueTable } from "@/components/KeyValueTable";
import { recordFromRows, rowsFromRecord } from "@/domain/rows";
import type { ScenarioTabProps } from "../types";

export function CaptureTab({ scenario, onChange }: ScenarioTabProps) {
  const rows = useMemo(() => rowsFromRecord(scenario.capture), [scenario.capture]);
  return (
    <>
      <KeyValueTable
        rows={rows}
        keyPlaceholder="Variable"
        valuePlaceholder="$.data.token  or  header:Location"
        onChange={(next) => onChange({ ...scenario, capture: recordFromRows(next) })}
      />
      <Hint>
        After each send, matching values are saved as variables (in .cactus/captured.json) so later requests can use
        them as {"{{name}}"}.
      </Hint>
    </>
  );
}
