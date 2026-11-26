import { describe, expect, it } from "vitest";
import { itemFields, listVariable } from "./itemFields";
import { wordSuggester } from "./suggestions";

describe("item fields", () => {
  it("lists keys, nested keys and lengths across items", () => {
    const json = JSON.stringify([
      { id: 1, tags: ["a"], user: { email: "x" } },
      { id: 2, extra: true },
    ]);
    expect(itemFields(json)).toEqual(["id", "tags", "tags.length", "user", "user.email", "user.email.length", "extra"]);
    expect(itemFields("not json")).toEqual([]);
    expect(itemFields('{"a": 1}')).toEqual([]);
    expect(itemFields("[1, 2]")).toEqual([]);
  });

  it("finds the variable a list names", () => {
    expect(listVariable(" {{ items }} ")).toBe("items");
    expect(listVariable("{{items}}.data")).toBeNull();
    expect(listVariable(undefined)).toBeNull();
  });

  it("suggests whole words, and stops once one is typed", () => {
    const suggest = wordSuggester(["id", "tags.length", "user.id"]);
    expect(suggest("id", 2).map((s) => s.insert)).toEqual([]);
    expect(suggest("i", 1).map((s) => s.insert)).toEqual(["id", "user.id"]);
    expect(suggest("", 0)).toHaveLength(3);
  });
});
