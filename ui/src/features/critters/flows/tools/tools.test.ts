import { describe, expect, it } from "vitest";
import { setField } from "@/domain/flowEdit";
import { generatorSuggester } from "@/domain/generators";
import { TOOL_IDS, TOOLS, toolDataflow } from ".";

describe("the tool registry", () => {
  it("creates each tool as the backend writes it: tagged, with no empty fields", () => {
    for (const id of TOOL_IDS) {
      const step = TOOLS[id].create();
      expect(step.tool).toBe(id);
      // Re-setting every field drops any the backend would leave out.
      const normalised = Object.entries(step).reduce<object>(
        (s, [key, value]) => setField(s as Record<string, unknown>, key, value),
        {},
      );
      expect(normalised, id).toEqual(step);
    }
  });

  it("describes what tools read and set", () => {
    expect(toolDataflow({ tool: "prompt", vars: [{ name: "code" }, {}] })).toEqual({
      templates: ["", "", "", "", ""],
      sets: ["code"],
      problems: ["A variable has no name"],
    });
    expect(toolDataflow({ tool: "set", values: { id: "$uuid", note: "{{id}}" } }).sets).toEqual(["id", "note"]);
    expect(toolDataflow({ tool: "webhook" }).problems).toEqual(["No URL"]);
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
    expect(toolDataflow({ tool: "poll" }).problems).toEqual(["No request chosen", "No Until checks"]);
    expect(toolDataflow({ tool: "run" }).problems).toEqual(["No flow chosen"]);
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

  it("suggests generators for a value starting with $", () => {
    expect(generatorSuggester("$r", 2).map((s) => s.insert)).toEqual(["$randomInt", "$randomEmail"]);
    expect(generatorSuggester("plain", 5)).toEqual([]);
  });
});
