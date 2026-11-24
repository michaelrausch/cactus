import { useMemo } from "react";
import { Hint } from "@/components/Field";
import { KeyValueTable } from "@/components/KeyValueTable";
import { parseQuery, withQuery } from "@/domain/query";
import type { ScenarioTabProps } from "../types";

export function ParamsTab({ scenario, onChange }: ScenarioTabProps) {
  const params = useMemo(() => parseQuery(scenario.url), [scenario.url]);
  return (
    <>
      <KeyValueTable rows={params} onChange={(rows) => onChange({ ...scenario, url: withQuery(scenario.url, rows) })} />
      <Hint>Kept in sync with the URL. Values are sent as written, so encode them yourself if needed.</Hint>
    </>
  );
}
