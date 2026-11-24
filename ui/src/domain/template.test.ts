import { describe, expect, it } from "vitest";
import { openPlaceholder, placeholderTail, templateParts } from "./template";

describe("templateParts", () => {
  it("finds placeholders the way the backend does", () => {
    expect(templateParts("{{base}}/x?a={{ t }}&b{{open")).toEqual([
      { text: "{{base}}", placeholder: "base" },
      { text: "/x?a=" },
      { text: "{{ t }}", placeholder: "t" },
      { text: "&b{{open" },
    ]);
  });

  it("gives back the text unchanged", () => {
    const text = "a {{b}}{{c}} }} {{";
    expect(
      templateParts(text)
        .map((part) => part.text)
        .join(""),
    ).toBe(text);
  });
});

describe("openPlaceholder", () => {
  it("reports the name being typed", () => {
    expect(openPlaceholder("{{ba", 4)).toEqual({ start: 0, query: "ba" });
    expect(openPlaceholder("x {{a}} {{", 10)).toEqual({ start: 8, query: "" });
  });

  it("ignores closed placeholders and other text", () => {
    expect(openPlaceholder("{{a}}", 5)).toBeNull();
    expect(openPlaceholder("{{a b", 5)).toBeNull();
    expect(openPlaceholder("plain", 5)).toBeNull();
  });

  it("measures the rest of a placeholder after the caret", () => {
    expect(placeholderTail("{{ba}}/x", 3)).toBe(3);
    expect(placeholderTail("{{b/x", 3)).toBe(0);
  });
});
