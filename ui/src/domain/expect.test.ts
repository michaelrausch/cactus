import { describe, expect, it } from "vitest";
import type { Scenario } from "@/api/client";
import {
  checkCount,
  expectJsonFromRows,
  expectRowsFromJson,
  formatStatus,
  parseStatusList,
  withExpect,
} from "./expect";

const base: Scenario = { title: "", description: "", method: "GET", url: "", headers: {}, body: null };

describe("expect", () => {
  it("parses status lists", () => {
    expect(parseStatusList("200")).toBe(200);
    expect(parseStatusList("200, 201 | 204")).toEqual([200, 201, 204]);
    expect(parseStatusList("99 600 abc")).toBeUndefined();
    expect(formatStatus([200, 201])).toBe("200, 201");
    expect(formatStatus(undefined)).toBe("");
  });

  it("keeps expected values typed through rows", () => {
    const json = { "$.ok": true, "$.code": "200", "$.n": 200 };
    expect(expectJsonFromRows(expectRowsFromJson(json))).toEqual(json);
  });

  it("drops empty parts", () => {
    const withStatus = withExpect(base, { status: 200 });
    expect(withStatus.expect).toEqual({ status: 200 });
    expect(checkCount(withStatus.expect)).toBe(1);
    const cleared = withExpect(withStatus, { status: undefined, json: {} });
    expect("expect" in cleared).toBe(false);
  });
});
