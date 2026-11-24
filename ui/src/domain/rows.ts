/** Editable key/value rows, as used by every table in the editor. */

export interface Row {
  /** Stable identity for React keys; never persisted. */
  id: number;
  key: string;
  value: string;
  enabled: boolean;
}

/** A `name`/`value` item of an OpenCollection list (headers, form fields, ...). */
export interface NamedValue {
  name: string;
  value: string;
  disabled?: boolean;
}

let lastRowId = 0;

export const makeRow = (key = "", value = "", enabled = true): Row => ({ id: ++lastRowId, key, value, enabled });

/** Rows without a key are placeholders the user is still typing into. */
export const isFilled = (row: Row): boolean => row.key.trim() !== "";

export const filledRows = (rows: Row[]): Row[] => rows.filter(isFilled);

/** Order-insensitive identity of the filled rows, so toggling a row doesn't make it jump. */
export const rowsFingerprint = (rows: Row[]): string =>
  JSON.stringify(
    filledRows(rows)
      .map((row) => [row.key, row.value, row.enabled])
      .toSorted(),
  );

export const rowsFromRecord = (record: Record<string, string> | undefined, enabled = true): Row[] =>
  Object.entries(record ?? {}).map(([key, value]) => makeRow(key, value, enabled));

export const recordFromRows = (rows: Row[]): Record<string, string> =>
  Object.fromEntries(rows.map((row) => [row.key, row.value]));

export const rowsFromList = (items: readonly NamedValue[] | undefined): Row[] =>
  (items ?? []).map((item) => makeRow(item.name, item.value, !item.disabled));

/** `disabled` is written only when set, so files stay minimal. */
export const listFromRows = (rows: Row[]): NamedValue[] =>
  rows.map((row) =>
    row.enabled ? { name: row.key, value: row.value } : { name: row.key, value: row.value, disabled: true },
  );
