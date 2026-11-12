import { describe, expect, it } from "vitest";
import { countdown, formatInterval, parseInterval } from "./interval";

describe("intervals", () => {
  it("parses what the backend accepts", () => {
    expect(parseInterval("30s")).toBe(30);
    expect(parseInterval(" 5m ")).toBe(300);
    expect(parseInterval("1h")).toBe(3600);
    expect(parseInterval("90")).toBe(90);
    for (const bad of ["29s", "0m", "", "m", "5d", "-5m", "1.5m", "99999999999h"]) {
      expect(parseInterval(bad), bad).toBeNull();
    }
  });

  it("writes the shortest unit", () => {
    expect([formatInterval(3600), formatInterval(300), formatInterval(90), formatInterval(120)]).toEqual([
      "1h",
      "5m",
      "90s",
      "2m",
    ]);
  });
});

describe("countdown", () => {
  it("counts down in m:ss, with hours when needed", () => {
    expect(countdown(192_000, 0)).toBe("in 3:12");
    expect(countdown(5_500, 0)).toBe("in 0:06");
    expect(countdown(3_723_000, 0)).toBe("in 1:02:03");
    expect(countdown(0, 10)).toBe("due");
  });
});
