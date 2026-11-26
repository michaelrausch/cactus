import { describe, expect, it } from "vitest";
import type { HttpResponse } from "@/api/client";
import { expressionSuggester, operatorSuggester, placeholderSuggester, type Suggestion } from "./suggestions";

const apply = (text: string, item: Suggestion | undefined) =>
  item ? text.slice(0, item.from) + item.insert + text.slice(item.to) : text;

const response = {
  body: JSON.stringify({ data: { token: "T", "odd key": 1, items: [{ id: 1 }] } }),
  headers: [
    ["Content-Type", "application/json"],
    ["ETag", "x"],
  ],
} as unknown as HttpResponse;

describe("placeholderSuggester", () => {
  const suggest = placeholderSuggester(["baseUrl", "token", "userToken"]);

  it("ranks names starting with the query first", () => {
    expect(suggest("{{tok", 5).map((s) => s.label)).toEqual(["token", "userToken"]);
  });

  it("says where each name comes from", () => {
    const withSources = placeholderSuggester(["baseUrl", "token"], { token: "Bob's context" });
    expect(withSources("{{", 2).map((s) => [s.label, s.detail])).toEqual([
      ["baseUrl", undefined],
      ["token", "Bob's context"],
    ]);
  });

  it("replaces the whole placeholder", () => {
    const text = "{{ba}}/users";
    expect(apply(text, suggest(text, 4)[0])).toBe("{{baseUrl}}/users");
    expect(apply("x {{", suggest("x {{", 4)[0])).toBe("x {{baseUrl}}");
  });
});

describe("expressionSuggester", () => {
  const suggest = expressionSuggester(response);
  const labels = (text: string) => suggest(text, text.length).map((s) => s.label);

  it("offers the response roots", () => {
    expect(labels("res.")).toEqual(["res.status", "res.body.", "res.headers.", "res.responseTime"]);
    expect(labels("res.s")).toEqual(["res.status"]);
    expect(labels("res.status")).toEqual([]);
  });

  it("offers body keys from the last response", () => {
    expect(labels("res.body.")).toEqual(["data"]);
    expect(labels("res.body.data.")).toEqual(["token", "odd key", "items"]);
    expect(labels("res.body.data.items.")).toEqual(["[0]"]);
    expect(labels("res.body.data.items[0].")).toEqual(["id"]);
    const odd = suggest("res.body.data.od", 16)[0];
    expect(apply("res.body.data.od", odd)).toBe("res.body.data['odd key']");
  });

  it("offers header names", () => {
    expect(labels("res.headers.")).toEqual(["content-type", "etag"]);
  });

  it("works without a response", () => {
    expect(expressionSuggester(null)("res.body.", 9)).toEqual([]);
  });
});

describe("operatorSuggester", () => {
  it("completes the operator and leaves the value", () => {
    expect(apply("con", operatorSuggester("con", 3)[0])).toBe("contains ");
    expect(apply("isD", operatorSuggester("isD", 3)[0])).toBe("isDefined");
    expect(apply("e 200", operatorSuggester("e 200", 1)[0])).toBe("eq 200");
  });

  it("stays quiet once an operator is typed", () => {
    expect(operatorSuggester("eq", 2)).toEqual([]);
    expect(operatorSuggester("eq 2", 4)).toEqual([]);
  });
});
