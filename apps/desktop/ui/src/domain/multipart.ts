/** Multipart form parts as editable rows. */
import type { MultipartField } from "@/api/client";
import { makeRow, type Row } from "./rows";

/** Several files in one part are shown, and typed, separated by this. */
const PATH_SEPARATOR = ", ";

const displayValue = (value: MultipartField["value"]): string =>
  Array.isArray(value) ? value.join(PATH_SEPARATOR) : value;

/** One row per part; the part's type is the row's `kind`. */
export const multipartRows = (fields: readonly MultipartField[]): Row[] =>
  fields.map((field) => ({
    ...makeRow(field.name, displayValue(field.value), !field.disabled),
    kind: field.type,
    origin: field,
  }));

/**
 * The parts the rows describe. A file part whose text wasn't edited keeps
 * its value exactly, several paths included; an edited one is split at commas
 * into several paths.
 */
export const multipartFromRows = (rows: readonly Row[]): MultipartField[] =>
  rows.map((row) => {
    const type = row.kind === "file" ? "file" : "text";
    const field: MultipartField = { name: row.key, type, value: partValue(row, type) };
    return row.enabled ? field : { ...field, disabled: true };
  });

function partValue(row: Row, type: MultipartField["type"]): MultipartField["value"] {
  const origin = row.origin as MultipartField | undefined;
  if (origin && origin.type === type && displayValue(origin.value) === row.value) return origin.value;
  if (type === "text") return row.value;
  const paths = row.value
    .split(",")
    .map((path) => path.trim())
    .filter(Boolean);
  return paths.length > 1 ? paths : row.value.trim();
}
