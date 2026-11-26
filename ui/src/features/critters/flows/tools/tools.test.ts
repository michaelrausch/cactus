import { describe, expect, it } from "vitest";
import { setField } from "@/domain/flowEdit";
import { generatorSuggester } from "@/domain/generators";
import { PRO } from "@edition";
import { TOOL_IDS, TOOL_SECTIONS, toolDataflow, toolDef } from ".";

describe("the tool registry", () => {
  it("creates each tool as the backend writes it: tagged, with no empty fields", () => {
    for (const id of TOOL_IDS) {
      const step = toolDef(id).create();
      expect(step.tool).toBe(id);
      // Re-setting every field drops any the backend would leave out.
      const normalised = Object.entries(step).reduce<object>(
        (s, [key, value]) => setField(s as Record<string, unknown>, key, value),
        {},
      );
      expect(normalised, id).toEqual(step);
    }
  });

  it("puts every tool in exactly one palette section", () => {
    const listed = TOOL_SECTIONS.flatMap((section) => section.tools);
    expect(listed.toSorted()).toEqual(TOOL_IDS.toSorted());
  });

  it.runIf(!PRO)("locks Pro tools in the free edition", () => {
    expect(TOOL_IDS).not.toContain("fuzz");
    expect(toolDef("fuzz")).toMatchObject({ label: "Fuzz", summary: expect.any(Function) });
    expect(toolDataflow({ tool: "fuzz" })).toEqual({ templates: [], sets: [], problems: [] });
  });

  it("describes what tools read and set", () => {
    expect(toolDataflow({ tool: "prompt", vars: [{ name: "code" }, {}] })).toEqual({
      templates: ["", "", "", "", ""],
      sets: ["code"],
      problems: ["A variable has no name"],
    });
    expect(toolDataflow({ tool: "set", values: { id: "$uuid", note: "{{id}}" } }).sets).toEqual(["id", "note"]);
    expect(toolDataflow({ tool: "wait", seconds: 601 }).problems).toEqual(["Wait at most 600 seconds"]);
    expect(toolDataflow({ tool: "notify", title: "{{a}}" })).toEqual({
      templates: ["{{a}}", ""],
      sets: [],
      problems: [],
    });
  });

  it("describes the flow-control tools", () => {
    expect(toolDataflow({ tool: "pick", list: "{{orders}}", name: "order_id" })).toEqual({
      templates: ["", "{{orders}}"],
      sets: ["order_id"],
      problems: [],
    });
    expect(toolDataflow({ tool: "pick" }).problems).toEqual(["No list", "No variable to set"]);
    const several = { tool: "pick" as const, list: "{{items}}", set: { item_code: "code", item_id: "id" } };
    expect(toolDataflow(several).sets).toEqual(["item_code", "item_id"]);
    const poll = toolDataflow({
      tool: "poll",
      request: "job.yml",
      with: { id: "{{job}}" },
      until: [{ expression: "res.body.status", operator: "eq", value: "ready" }],
      capture: { result: "res.body.url" },
    });
    expect(poll).toEqual({ templates: ["{{job}}", "ready"], sets: ["result"], problems: [] });
    expect(toolDataflow({ tool: "poll" }).problems).toEqual(["No request", "No Until checks"]);
    expect(toolDataflow({ tool: "run" }).problems).toEqual(["No flow"]);
    const check = { expression: "{{items}}.length", operator: "gt", value: "{{min}}" };
    const itemCheck = { expression: "id", operator: "isNotEmpty" };
    expect(toolDataflow({ tool: "assert", checks: [check], each: { list: "{{items}}", checks: [itemCheck] } })).toEqual(
      {
        templates: ["{{items}}.length", "{{items}}", "{{min}}", ""],
        sets: [],
        problems: [],
      },
    );
    expect(toolDataflow({ tool: "assert", each: { list: "{{items}}" } }).problems).toEqual(["No checks for each item"]);
  });

  it.runIf(PRO)("describes the security tools", () => {
    expect(toolDataflow({ tool: "webhook" }).problems).toEqual(["No URL"]);
    expect(toolDataflow({ tool: "fuzz" }).problems).toEqual(["No request"]);
    expect(toolDataflow({ tool: "fuzz", request: "a.yml", with: { id: "{{id}}" }, concurrency: 33 })).toEqual({
      templates: ["{{id}}"],
      sets: [],
      problems: ["Send at most 32 at once"],
    });
    expect(toolDataflow({ tool: "cert" }).problems).toEqual(["No URL"]);
    expect(toolDataflow({ tool: "headers" }).problems).toEqual(["No request"]);
    expect(toolDataflow({ tool: "headers", request: "a.yml", set: { issues: "findings", x: "p95" } })).toEqual({
      templates: [],
      sets: ["issues", "x"],
      problems: ["Set x: no p95"],
    });
    const check = { expression: "days_left", operator: "gt", value: "{{min}}" };
    expect(toolDataflow({ tool: "cert", url: "{{host}}", checks: [check] }).templates).toEqual(["{{host}}", "{{min}}"]);
    expect(toolDataflow({ tool: "ratelimit", request: "a.yml", limit: 400 }).problems).toEqual([]);
    expect(toolDataflow({ tool: "ratelimit", request: "a.yml", limit: 10, requests: 10 }).problems).toEqual([
      "Send more requests than the limit",
    ]);
    expect(toolDataflow({ tool: "loadtest", request: "a.yml", duration: 601 }).problems).toEqual([
      "Run for at most 600 seconds",
    ]);
  });

  it.runIf(PRO)("hands findings and metrics on to later steps", () => {
    const fuzz = toolDataflow({ tool: "fuzz", request: "a.yml", set: { issues: "findings", highs: "findings.high" } });
    expect(fuzz).toEqual({ templates: [], sets: ["issues", "highs"], problems: [] });
    expect(toolDataflow({ tool: "loadtest", request: "a.yml", set: { latency: "p95" } }).sets).toEqual(["latency"]);
    expect(toolDataflow({ tool: "loadtest", request: "a.yml", set: { x: "findings" } }).problems).toEqual([
      "Set x: no findings",
    ]);
    expect(toolDataflow({ tool: "cert", url: "h", set: { days: "days_left", n: "findings.count" } }).problems).toEqual(
      [],
    );
  });

  it("suggests generators for a value starting with $", () => {
    expect(generatorSuggester("$r", 2).map((s) => s.insert)).toEqual(["$randomInt", "$randomEmail"]);
    expect(generatorSuggester("plain", 5)).toEqual([]);
  });
});
