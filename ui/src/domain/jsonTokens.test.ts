import { describe, expect, it } from "vitest";
import { jsonTokens } from "./jsonTokens";

const kinds = (text: string) => jsonTokens(text).map((token) => [token.kind, token.text]);

describe("jsonTokens", () => {
  it("tells keys from values", () => {
    expect(kinds('{"a": "b", "n": -1.5e3, "ok": true}')).toEqual([
      ["punctuation", "{"],
      ["key", '"a"'],
      ["punctuation", ":"],
      ["text", " "],
      ["string", '"b"'],
      ["punctuation", ","],
      ["text", " "],
      ["key", '"n"'],
      ["punctuation", ":"],
      ["text", " "],
      ["number", "-1.5e3"],
      ["punctuation", ","],
      ["text", " "],
      ["key", '"ok"'],
      ["punctuation", ":"],
      ["text", " "],
      ["literal", "true"],
      ["punctuation", "}"],
    ]);
  });

  it("handles placeholders, escapes and unfinished text", () => {
    expect(kinds('{"n": {{count}}}')).toContainEqual(["placeholder", "{{count}}"]);
    expect(kinds('"a\\"b"')).toEqual([["string", '"a\\"b"']]);
    expect(kinds('{"open\n}')).toEqual([
      ["punctuation", "{"],
      ["string", '"open'],
      ["text", "\n"],
      ["punctuation", "}"],
    ]);
  });

  it("gives back the text unchanged", () => {
    const text = '{\n  "k": [1, null, "{{v}}"], oops {{x\n}';
    expect(
      jsonTokens(text)
        .map((token) => token.text)
        .join(""),
    ).toBe(text);
  });
});
