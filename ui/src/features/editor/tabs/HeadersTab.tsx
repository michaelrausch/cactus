import { useMemo } from "react";
import { Hint } from "@/components/Field";
import { KeyValueTable } from "@/components/KeyValueTable";
import { recordFromRows, rowsFromRecord, type Row } from "@/domain/rows";
import type { ScenarioTabProps } from "../types";

export function HeadersTab({ scenario, onChange }: ScenarioTabProps) {
  const rows = useMemo(
    () => [...rowsFromRecord(scenario.headers), ...rowsFromRecord(scenario.disabledHeaders, false)],
    [scenario.headers, scenario.disabledHeaders],
  );
  const update = (next: Row[]) =>
    onChange({
      ...scenario,
      headers: recordFromRows(next.filter((row) => row.enabled)),
      disabledHeaders: recordFromRows(next.filter((row) => !row.enabled)),
    });
  return (
    <>
      <KeyValueTable rows={rows} toggles onChange={update} />
      <Hint>Unchecked headers stay in the file but aren't sent. An explicit Authorization header overrides Auth.</Hint>
    </>
  );
}
