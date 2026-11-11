import { describe, expect, it } from "vitest";
import { groupByFolder } from "./entries";

const entry = (relPath: string, title = relPath, method = "GET") => ({ relPath, title, method, tags: [] });

describe("groupByFolder", () => {
  const entries = [entry("a.json"), entry("auth/login.json", "Login", "POST"), entry("auth/me.json")];

  it("groups by folder in order", () => {
    expect(groupByFolder(entries, "").map((g) => [g.dir, g.entries.length])).toEqual([
      ["", 1],
      ["auth", 2],
    ]);
  });

  it("filters on title, path and method", () => {
    expect(groupByFolder(entries, "post").flatMap((g) => g.entries.map((e) => e.relPath))).toEqual(["auth/login.json"]);
    expect(groupByFolder(entries, "nothing")).toEqual([]);
  });
});
