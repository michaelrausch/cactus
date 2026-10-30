import { useMemo } from "react";
import { Hint } from "@/components/Field";
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
      <Hint>
        Checked on every send and when running a folder. Read <code>res.status</code>, <code>res.body.path</code>,{" "}
        <code>res.headers.name</code> or <code>res.responseTime</code>, then an operator and a value: eq, neq, gt, gte,
        lt, lte, in, notIn, contains, startsWith, endsWith, matches, length, isDefined, isNull, isEmpty, isString,
        isNumber, isArray, isJson, isTruthy. Without a status assertion, any status under 400 passes.
      </Hint>
    </>
  );
}
