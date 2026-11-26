import type { Action, Assertion, Scenario, SetVariable } from "@/api/client";
import { ASSERTION_OPERATORS } from "./http";
import { makeRow, type Row } from "./rows";

const DEFAULT_OPERATOR = "eq";
const isOperator = (word: string): boolean => (ASSERTION_OPERATORS as readonly string[]).includes(word);

/** Shown as Bruno shows them: `res.status` | `eq 200`. */
export const rowsFromAssertions = (assertions: readonly Assertion[] | undefined): Row[] =>
  (assertions ?? []).map((a) =>
    makeRow(a.expression, a.value === undefined ? a.operator : `${a.operator} ${a.value}`, !a.disabled),
  );

/** The inverse of {@link rowsFromAssertions}; `disabled` is written only when set. */
export const assertionsFromRows = (rows: Row[]): Assertion[] =>
  rows.map((row) => ({
    expression: row.key,
    ...parseCondition(row.value),
    ...(row.enabled ? {} : { disabled: true }),
  }));

export const assertionRows = (scenario: Scenario): Row[] => rowsFromAssertions(scenario.runtime?.assertions);

/** `"eq 200"` → `eq` and `"200"`; text without a known operator means `eq` it. */
export function parseCondition(text: string): Pick<Assertion, "operator" | "value"> {
  const trimmed = text.trim();
  const [first = "", ...rest] = trimmed.split(/\s+/);
  if (!isOperator(first)) return { operator: DEFAULT_OPERATOR, value: trimmed };
  const value = trimmed.slice(first.length).trim();
  return rest.length > 0 ? { operator: first, value } : { operator: first };
}

const withRuntime = (scenario: Scenario, patch: Partial<NonNullable<Scenario["runtime"]>>): Scenario => {
  const runtime = { ...scenario.runtime, ...patch };
  const kept = Object.fromEntries(
    Object.entries(runtime).filter(([, list]) => Array.isArray(list) && list.length > 0),
  ) as NonNullable<Scenario["runtime"]>;
  const { runtime: _previous, ...rest } = scenario;
  return Object.keys(kept).length > 0 ? { ...rest, runtime: kept } : rest;
};

export const withAssertionRows = (scenario: Scenario, rows: Row[]): Scenario =>
  withRuntime(scenario, { assertions: assertionsFromRows(rows) });

export const activeAssertionCount = (scenario: Scenario): number =>
  (scenario.runtime?.assertions ?? []).filter((a) => !a.disabled).length;

const isSetVariable = (action: Action): action is SetVariable =>
  typeof action === "object" && action !== null && !Array.isArray(action) && action["type"] === "set-variable";

const setVariables = (scenario: Scenario): SetVariable[] => (scenario.runtime?.actions ?? []).filter(isSetVariable);

/** Set-variable actions as `name` | `expression` rows, each carrying its action. */
export const varRows = (scenario: Scenario): Row[] =>
  setVariables(scenario).map((set) =>
    Object.assign(makeRow(set.variable.name, set.selector.expression, !set.disabled), { origin: set }),
  );

/**
 * Rows become set-variable actions. A row made from an action keeps that
 * action's phase, scope and selector method; other actions stay.
 */
export function withVarRows(scenario: Scenario, rows: Row[]): Scenario {
  const others = (scenario.runtime?.actions ?? []).filter((action) => !isSetVariable(action));
  const sets = rows.map((row): SetVariable => {
    const base = row.origin as SetVariable | undefined;
    const { disabled: _previous, ...kept } = base ?? {
      type: "set-variable",
      phase: "after-response",
      selector: { expression: "", method: "jsonq" },
      variable: { name: "", scope: "runtime" },
    };
    return {
      ...kept,
      selector: { ...kept.selector, expression: row.value },
      variable: { ...kept.variable, name: row.key },
      ...(row.enabled ? {} : { disabled: true }),
    };
  });
  return withRuntime(scenario, { actions: [...sets, ...others] });
}

export const varCount = (scenario: Scenario): number => setVariables(scenario).filter((set) => !set.disabled).length;
