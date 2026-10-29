import { describe, expect, it } from "vitest";
import { makeRow, recordFromRows, rowsFingerprint, rowsFromRecord } from "./rows";

describe("rows", () => {
  it("gives every row a distinct id", () => {
    expect(makeRow().id).not.toBe(makeRow().id);
  });

  it("round-trips records", () => {
    const record = { a: "1", b: "2" };
    expect(recordFromRows(rowsFromRecord(record))).toEqual(record);
    expect(rowsFromRecord(undefined)).toEqual([]);
  });

  it("fingerprints ignore order, ids and blank rows", () => {
    const a = [makeRow("x", "1"), makeRow("y", "2"), makeRow()];
    const b = [makeRow("y", "2"), makeRow("x", "1")];
    expect(rowsFingerprint(a)).toBe(rowsFingerprint(b));
    expect(rowsFingerprint(a)).not.toBe(rowsFingerprint([makeRow("x", "1", false), makeRow("y", "2")]));
  });
});
