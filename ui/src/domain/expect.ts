import type { Expect, JsonValue, Scenario, StatusExpect } from "@/api/client";
import { makeRow, type Row } from "./rows";
import { parseLooseJson } from "./scenario";

const MIN_HTTP_STATUS = 100;
const MAX_HTTP_STATUS = 599;
const STATUS_SEPARATORS = /[\s,|]+/;

const isHttpStatus = (n: number): boolean => Number.isInteger(n) && n >= MIN_HTTP_STATUS && n <= MAX_HTTP_STATUS;

/** `"200, 201"` → `[200, 201]`, `"200"` → `200`, nothing valid → `undefined`. */
export function parseStatusList(text: string): StatusExpect | undefined {
  const codes = text.split(STATUS_SEPARATORS).map(Number).filter(isHttpStatus);
  if (codes.length === 0) return undefined;
  return codes.length === 1 ? codes[0] : codes;
}

export function formatStatus(status: StatusExpect | undefined): string {
  if (status === undefined) return "";
  return Array.isArray(status) ? status.join(", ") : String(status);
}

/** Expected values are shown as JSON so `"200"` and `200` stay distinguishable. */
export const expectRowsFromJson = (json: Expect["json"]): Row[] =>
  Object.entries(json ?? {}).map(([path, value]) => makeRow(path, JSON.stringify(value)));

export const expectJsonFromRows = (rows: Row[]): Record<string, JsonValue> =>
  Object.fromEntries(rows.map((row) => [row.key, parseLooseJson(row.value)]));

export interface ExpectPatch {
  status?: StatusExpect | undefined;
  json?: Record<string, JsonValue> | undefined;
}

/** Apply `patch`, dropping empty parts so the file stays minimal. */
export function withExpect(scenario: Scenario, patch: ExpectPatch): Scenario {
  const merged = { ...scenario.expect, ...patch };
  const expect: Expect = {};
  if (merged.status !== undefined) expect.status = merged.status;
  if (merged.json && Object.keys(merged.json).length > 0) expect.json = merged.json;

  const { expect: _previous, ...rest } = scenario;
  return Object.keys(expect).length > 0 ? { ...rest, expect } : rest;
}

export const checkCount = (expect: Expect | undefined): number =>
  (expect?.status === undefined ? 0 : 1) + Object.keys(expect?.json ?? {}).length;
