import { describe, expect, it } from "vitest";
import { describePlaceholders, openPlaceholder, placeholderTail, templateParts } from "./template";

describe("describePlaceholders", () => {
  it("lists each placeholder once with its source", () => {
    const details = { baseUrl: "staging", token: "Bob's context · auth/login.yml, 2m ago" };
    expect(describePlaceholders("{{baseUrl}}/me?t={{ token }}&again={{baseUrl}}&x={{nope}}", details)).toBe(
      "baseUrl: staging\ntoken: Bob's context · auth/login.yml, 2m ago\nnope: not set",
    );
    expect(describePlaceholders("plain", details)).toBe("");
  });
});

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
