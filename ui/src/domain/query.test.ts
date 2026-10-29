import { describe, expect, it } from "vitest";
import { parseQuery, withQuery } from "./query";
import { makeRow } from "./rows";

const pairs = (url: string) => parseQuery(url).map((row) => [row.key, row.value]);

describe("query", () => {
  it("parses without decoding", () => {
    expect(pairs("{{base}}/x?a=1&b=%20&flag#frag")).toEqual([
      ["a", "1"],
      ["b", "%20"],
      ["flag", ""],
    ]);
    expect(pairs("http://h/x")).toEqual([]);
  });

  it("rewrites the query and keeps base and hash", () => {
    const rows = [makeRow("q", "{{term}}"), makeRow("flag"), makeRow("", "ignored")];
    expect(withQuery("http://h/x?old=1#top", rows)).toBe("http://h/x?q={{term}}&flag#top");
    expect(withQuery("http://h/x?old=1", [])).toBe("http://h/x");
  });
});
