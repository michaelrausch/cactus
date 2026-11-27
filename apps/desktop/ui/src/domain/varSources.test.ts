import { describe, expect, it } from "vitest";
import { overriddenNames, sourceDetails, varSourceDetail, varSourceLabel } from "./varSources";

describe("variable sources", () => {
  it("labels every layer", () => {
    expect(varSourceLabel({ kind: "collection" })).toBe("shared");
    expect(varSourceLabel({ kind: "environment", name: "staging" })).toBe("staging");
    expect(varSourceLabel({ kind: "folder", dir: "auth" })).toBe("folder auth");
    expect(varSourceLabel({ kind: "critterSecret" }, "Bob")).toBe("Bob's secrets");
    expect(varSourceLabel({ kind: "critterContext", from: "auth/login.yml", at: 0 }, "Bob")).toBe(
      "Bob's context · auth/login.yml",
    );
    expect(varSourceLabel({ kind: "critter" })).toBe("bot variables");
  });

  it("adds the age of captured values", () => {
    const captured = { kind: "critterContext", from: "auth/login.yml", at: 0 } as const;
    expect(varSourceDetail(captured, "Bob", 120_000)).toBe("Bob's context · auth/login.yml, 2m ago");
    expect(sourceDetails({ base: { kind: "environment", name: "staging" } }, "Bob", 0)).toEqual({ base: "staging" });
  });

  it("lists what a critter overrides", () => {
    const lower = { BASE_URL: { kind: "environment", name: "staging" }, TOKEN: { kind: "secret" } } as const;
    expect(overriddenNames(["TOKEN", "CLIENT_ID", "BASE_URL"], lower)).toEqual([
      ["BASE_URL", "staging"],
      ["TOKEN", "project secrets"],
    ]);
  });
});
