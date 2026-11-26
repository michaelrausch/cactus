import { useMemo } from "react";
import { KeyValueTable } from "@/components/KeyValueTable";
import { assertionRows, withAssertionRows } from "@/domain/assertions";
import { operatorSuggester } from "@/domain/suggestions";
import { useExpressionSuggester } from "@/hooks/useExpressionSuggester";
import type { ScenarioTabProps } from "../types";

export function AssertionsTab({ scenario, onChange }: ScenarioTabProps) {
  const rows = useMemo(() => assertionRows(scenario), [scenario]);
  const suggestExpression = useExpressionSuggester();
  return (
    <>
      <KeyValueTable
        rows={rows}
        toggles
        keyPlaceholder="res.status"
        valuePlaceholder="eq 200"
        keyColumn={{ suggest: suggestExpression }}
        valueColumn={{ suggest: operatorSuggester }}
        echo={(next) => assertionRows(withAssertionRows(scenario, next))}
        onChange={(next) => onChange(withAssertionRows(scenario, next))}
      />
    </>
  );
}
