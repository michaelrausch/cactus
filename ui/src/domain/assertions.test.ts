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

describe("assertions", () => {
  it("parses conditions like Bruno writes them", () => {
    expect(parseCondition("eq 200")).toEqual({ operator: "eq", value: "200" });
    expect(parseCondition("in 200, 201")).toEqual({ operator: "in", value: "200, 201" });
    expect(parseCondition("isDefined")).toEqual({ operator: "isDefined" });
    expect(parseCondition("hello world")).toEqual({ operator: "eq", value: "hello world" });
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
    const next = withVarRows(s, [makeRow("a2", "res.body.a2"), makeRow("b", "res.headers.etag", false)]);
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
});
