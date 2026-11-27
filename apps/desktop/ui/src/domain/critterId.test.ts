import { describe, expect, it } from "vitest";
import {
  critterIdFrom,
  critterIdOf,
  critterNameProblem,
  flowFilePath,
  flowOfPath,
  isCritterPath,
  isValidCritterId,
  uniqueCritterId,
} from "./critterId";

describe("critter ids", () => {
  it("slugs names into valid ids", () => {
    expect(critterIdFrom("Bob")).toBe("bob");
    expect(critterIdFrom("  Bob (staging) ")).toBe("bob-staging");
    expect(critterIdFrom("Zoë_admin")).toBe("zoe_admin");
    expect(critterIdFrom("../../etc")).toBe("etc");
    expect(critterIdFrom("🦎")).toBe("critter");
    for (const name of ["Bob", "a b/c", "🦎", "x..y"]) expect(isValidCritterId(critterIdFrom(name))).toBe(true);
    expect(isValidCritterId("a/b")).toBe(false);
    expect(isValidCritterId("")).toBe(false);
  });

  it("picks a free id", () => {
    expect(uniqueCritterId("Bob", [])).toBe("bob");
    expect(uniqueCritterId("Bob", ["bob"])).toBe("bob-2");
    expect(uniqueCritterId("Bob", ["BOB", "bob-2"])).toBe("bob-3");
  });

  it("recognises critter files", () => {
    expect(isCritterPath("critters/bob/critter.yml")).toBe(true);
    expect(isCritterPath("critters/bob/flows/login.yml")).toBe(true);
    expect(isCritterPath("critters.yml")).toBe(false);
    expect(isCritterPath("api/critters/x.yml")).toBe(false);
    expect(critterIdOf("critters/bob/flows/login.yml")).toBe("bob");
    expect(critterIdOf("critters/bob")).toBe("bob");
    expect(critterIdOf("critters/")).toBeNull();
    expect(critterIdOf("auth/login.yml")).toBeNull();
    expect(flowFilePath("bob", "health")).toBe("critters/bob/flows/health.yml");
    expect(flowOfPath("critters/bob/flows/health.yml")).toEqual({ critter: "bob", flow: "health" });
    expect(flowOfPath("critters/bob/critter.yml")).toBeNull();
    expect(flowOfPath("critters/bob/flows/a/b.yml")).toBeNull();
  });

  it("explains names that can't be used", () => {
    const others = [{ name: "Alice" }];
    expect(critterNameProblem("  ", others)).toBe("Give it a name.");
    expect(critterNameProblem("alice ", others)).toBe("There's already a bot called alice.");
    expect(critterNameProblem("Bob", others)).toBeNull();
  });
});
