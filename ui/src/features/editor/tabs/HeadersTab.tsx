import { useMemo } from "react";
import { Hint } from "@/components/Field";
import { KeyValueTable } from "@/components/KeyValueTable";
import { listFromRows, rowsFromList } from "@/domain/rows";
import type { ScenarioTabProps } from "../types";

export function HeadersTab({ scenario, onChange }: ScenarioTabProps) {
  const rows = useMemo(() => rowsFromList(scenario.http.headers), [scenario.http.headers]);
  return (
    <>
      <KeyValueTable
        rows={rows}
        toggles
        onChange={(next) => onChange({ ...scenario, http: { ...scenario.http, headers: listFromRows(next) } })}
      />
      <Hint>
        Unchecked headers stay in the file but aren&apos;t sent. Headers set on the collection or a folder are sent too;
        one here with the same name wins. An explicit Authorization header overrides Auth.
      </Hint>
    </>
  );
}
