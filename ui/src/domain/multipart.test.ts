import { describe, expect, it } from "vitest";
import type { MultipartField } from "@/api/client";
import { multipartFromRows, multipartRows } from "./multipart";

const fields: MultipartField[] = [
  { name: "handle", type: "text", value: "@me" },
  { name: "photos", type: "file", value: ["a.png", "b.png"] },
  { name: "doc", type: "file", value: "c.pdf", disabled: true },
];

describe("multipart rows", () => {
  it("round-trip every part unchanged", () => {
    expect(multipartFromRows(multipartRows(fields))).toEqual(fields);
  });

  it("keep the part's type with the row, whatever its text", () => {
    const rows = multipartRows(fields).map((row) =>
      row.key === "handle" ? Object.assign(row, { value: "@you" }) : row,
    );
    expect(multipartFromRows(rows)[0]).toEqual({ name: "handle", type: "text", value: "@you" });
  });

  it("split an edited file part into its paths", () => {
    const rows = multipartRows(fields).map((row) =>
      row.key === "photos" ? Object.assign(row, { value: "a.png, d.png" }) : row,
    );
    expect(multipartFromRows(rows)[1]?.value).toEqual(["a.png", "d.png"]);
  });

  it("change a part's type with the row's kind", () => {
    const rows = multipartRows(fields).map((row) => (row.key === "doc" ? Object.assign(row, { kind: "text" }) : row));
    expect(multipartFromRows(rows)[2]).toEqual({ name: "doc", type: "text", value: "c.pdf", disabled: true });
  });
});
