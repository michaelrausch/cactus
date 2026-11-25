import { describe, expect, it } from "vitest";
import { ACHIEVEMENTS, newlyUnlocked } from "./achievements";

const ids = (achievements: { id: string }[]) => achievements.map((achievement) => achievement.id);

describe("newlyUnlocked", () => {
  it("unlocks nothing from nothing", () => {
    expect(newlyUnlocked({}, {})).toEqual([]);
  });

  it("unlocks every goal a count reaches", () => {
    expect(ids(newlyUnlocked({ requestsSent: 1000 }, {}))).toEqual(["requests-100", "requests-1000"]);
  });

  it("skips what is already unlocked", () => {
    expect(ids(newlyUnlocked({ requestsSent: 1000 }, { "requests-100": "2026-01-01" }))).toEqual(["requests-1000"]);
  });

  it("waits until the goal is reached", () => {
    expect(ids(newlyUnlocked({ requestsSent: 999 }, { "requests-100": "x" }))).toEqual([]);
  });
});

describe("ACHIEVEMENTS", () => {
  it("have unique ids", () => {
    expect(new Set(ids([...ACHIEVEMENTS])).size).toBe(ACHIEVEMENTS.length);
  });
});
