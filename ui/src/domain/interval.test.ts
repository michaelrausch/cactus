import { describe, expect, it } from "vitest";
import { countdown } from "./interval";

describe("countdown", () => {
  it("counts down in m:ss, with hours when needed", () => {
    expect(countdown(192_000, 0)).toBe("in 3:12");
    expect(countdown(5_500, 0)).toBe("in 0:06");
    expect(countdown(3_723_000, 0)).toBe("in 1:02:03");
    expect(countdown(0, 10)).toBe("due");
  });
});
