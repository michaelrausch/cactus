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

  it("shows empty folders after the last group inside their parent", () => {
    const groups = groupByFolder(entries, "", ["auth", "auth/admin", "users", "zzz"]);
    expect(groups.map((g) => [g.dir, g.entries.length])).toEqual([
      ["", 1],
      ["auth", 2],
      ["auth/admin", 0],
      ["users", 0],
      ["zzz", 0],
    ]);
    expect(groupByFolder(entries, "login", ["users"]).map((g) => g.dir)).toEqual(["auth"]);
  });

  it("filters on title, path and method", () => {
    expect(groupByFolder(entries, "post").flatMap((g) => g.entries.map((e) => e.relPath))).toEqual(["auth/login.json"]);
    expect(groupByFolder(entries, "nothing")).toEqual([]);
  });
});
