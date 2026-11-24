import { describe, expect, it } from "vitest";
import type { Scenario } from "@/api/client";
import {
  activeAssertionCount,
  assertionRows,
  parseCondition,
  varCount,
  varRows,
  withAssertionRows,
  withVarRows,
} from "./assertions";
import { makeRow } from "./rows";

const scenario = (runtime?: Scenario["runtime"]): Scenario => ({
  info: { name: "", type: "http" },
  http: { method: "GET", url: "" },
  ...(runtime ? { runtime } : {}),
});

/** A condition as the assertions table shows it once stored. */
const shown = (value: string) => assertionRows(withAssertionRows(scenario(), [makeRow("res.status", value)]))[0]?.value;

describe("assertions", () => {
  it("parses conditions like Bruno writes them", () => {
    expect(parseCondition("eq 200")).toEqual({ operator: "eq", value: "200" });
    expect(parseCondition("in 200, 201")).toEqual({ operator: "in", value: "200, 201" });
    expect(parseCondition("isDefined")).toEqual({ operator: "isDefined" });
    expect(parseCondition("hello world")).toEqual({ operator: "eq", value: "hello world" });
  });

  // The table is told about this normalisation (KeyValueTable's `echo`) so it
  // doesn't reset half-typed text: before, "eq " came back as "eq" and the next
  // keystroke made "eq2", saved as `eq eq2`.
  it("normalises half-typed conditions when stored", () => {
    expect(shown("eq ")).toBe("eq");
    expect(shown("200")).toBe("eq 200");
    expect(shown("eq  2")).toBe("eq 2");
  });

  it("round-trips rows", () => {
    const s = withAssertionRows(scenario(), [
      makeRow("res.status", "eq 200"),
      makeRow("res.body.id", "isNumber", false),
    ]);
    expect(s.runtime?.assertions).toEqual([
      { expression: "res.status", operator: "eq", value: "200" },
      { expression: "res.body.id", operator: "isNumber", disabled: true },
    ]);
    expect(assertionRows(s).map((r) => [r.key, r.value, r.enabled])).toEqual([
      ["res.status", "eq 200", true],
      ["res.body.id", "isNumber", false],
    ]);
    expect(activeAssertionCount(s)).toBe(1);
    expect("runtime" in withAssertionRows(s, [])).toBe(false);
  });
});

const setVariable = (name: string, scope: string) => ({
  type: "set-variable" as const,
  phase: "after-response" as const,
  selector: { expression: `res.body.${name}`, method: "jsonq" },
  variable: { name, scope },
});

describe("vars", () => {
  it("keeps phase, scope and other actions", () => {
    const other = { type: "something-else" };
    const s = scenario({
      actions: [
        {
          type: "set-variable",
          phase: "after-response",
          selector: { expression: "res.body.a", method: "jsonq" },
          variable: { name: "a", scope: "environment" },
        },
        other,
      ],
      scripts: [{ type: "tests", code: "x" }],
    });
    expect(varRows(s).map((r) => [r.key, r.value])).toEqual([["a", "res.body.a"]]);
    const edited = varRows(s).map((row) => Object.assign(row, { key: "a2", value: "res.body.a2" }));
    const next = withVarRows(s, [...edited, makeRow("b", "res.headers.etag", false)]);
    expect(next.runtime?.actions).toEqual([
      {
        type: "set-variable",
        phase: "after-response",
        selector: { expression: "res.body.a2", method: "jsonq" },
        variable: { name: "a2", scope: "environment" },
      },
      {
        type: "set-variable",
        phase: "after-response",
        selector: { expression: "res.headers.etag", method: "jsonq" },
        variable: { name: "b", scope: "runtime" },
        disabled: true,
      },
      other,
    ]);
    expect(next.runtime?.scripts).toEqual([{ type: "tests", code: "x" }]);
    expect(varCount(next)).toBe(1);
  });

  it("keeps each action's settings with its own row when an earlier row is removed", () => {
    const s = scenario({ actions: [setVariable("a", "environment"), setVariable("b", "runtime")] });
    const next = withVarRows(s, varRows(s).slice(1));
    expect(next.runtime?.actions).toEqual([setVariable("b", "runtime")]);
  });
});
