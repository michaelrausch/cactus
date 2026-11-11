import { describe, expect, it } from "vitest";
import { insertionIndex, moveTarget } from "./dragInsert";

describe("drag insertion", () => {
  const midpoints = [20, 60, 100];

  it("inserts before the first card below the pointer", () => {
    expect(insertionIndex(0, midpoints)).toBe(0);
    expect(insertionIndex(30, midpoints)).toBe(1);
    expect(insertionIndex(99, midpoints)).toBe(2);
    expect(insertionIndex(500, midpoints)).toBe(3);
    expect(insertionIndex(10, [])).toBe(0);
  });

  it("turns an insertion point into a move", () => {
    expect(moveTarget(0, 3)).toBe(2);
    expect(moveTarget(2, 0)).toBe(0);
    expect(moveTarget(1, 1)).toBeNull();
    expect(moveTarget(1, 2)).toBeNull();
  });
});
