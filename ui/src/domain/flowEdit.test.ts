import { describe, expect, it } from "vitest";
import type { Flow } from "@/api/client";
import {
  insertStep,
  moveStep,
  removeStep,
  stepAssertRows,
  stepMapRows,
  updateStep,
  withCapture,
  withExpect,
  withOverride,
  withStepAssertRows,
  withStepMapRows,
  withWatch,
} from "./flowEdit";
import { makeRow } from "./rows";

const flow = (...requests: string[]): Flow => ({ name: "F", steps: requests.map((request) => ({ request })) });
const requests = (f: Flow) => f.steps.map((s) => s.request);

describe("flow edits", () => {
  it("inserts, moves and removes steps", () => {
    expect(requests(insertStep(flow("a", "c"), 1, "b"))).toEqual(["a", "b", "c"]);
    expect(requests(insertStep(flow("a"), 99, "b"))).toEqual(["a", "b"]);
    expect(requests(moveStep(flow("a", "b", "c"), 0, 2))).toEqual(["b", "c", "a"]);
    expect(requests(moveStep(flow("a", "b", "c"), 2, 0))).toEqual(["c", "a", "b"]);
    expect(requests(removeStep(flow("a", "b"), 0))).toEqual(["b"]);
    expect(updateStep(flow("a", "b"), 1, (s) => withExpect(s, 403)).steps[1]).toEqual({ request: "b", expect: 403 });
  });

  it("leaves empty fields out", () => {
    const step = { request: "a.yml", expect: 200 };
    expect(withExpect(step, null)).toEqual({ request: "a.yml" });
    expect(withStepMapRows(step, "with", [makeRow("", "x")])).toEqual(step);
    expect(withWatch(step, [" ", ""])).toEqual(step);
    expect(withStepAssertRows(step, [])).toEqual(step);
  });

  it("round-trips maps, checks and watches through rows", () => {
    const step = withStepMapRows({ request: "a" }, "capture", [makeRow("token", "res.body.token")]);
    expect(step.capture).toEqual({ token: "res.body.token" });
    expect(stepMapRows(step, "capture").map((r) => [r.key, r.value])).toEqual([["token", "res.body.token"]]);
    const checked = withStepAssertRows({ request: "a" }, [
      makeRow("res.body.db", "eq ok"),
      makeRow("res.status", "lt 500", false),
    ]);
    expect(checked.assert).toEqual([
      { expression: "res.body.db", operator: "eq", value: "ok" },
      { expression: "res.status", operator: "lt", value: "500", disabled: true },
    ]);
    expect(stepAssertRows(checked).map((r) => r.value)).toEqual(["eq ok", "lt 500"]);
    expect(withWatch({ request: "a" }, [" res.body.version "]).watch).toEqual(["res.body.version"]);
  });

  it("adds single captures and overrides", () => {
    expect(withCapture({ request: "a" }, "id", "res.body.id").capture).toEqual({ id: "res.body.id" });
    expect(withOverride({ request: "a", with: { a: "1" } }, "b").with).toEqual({ a: "1", b: "" });
  });
});
