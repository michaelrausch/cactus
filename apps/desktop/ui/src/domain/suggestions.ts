/** Autocomplete for `{{name}}` placeholders and response expressions. */
import type { HttpResponse } from "@/api/client";
import { ASSERTION_OPERATORS } from "./http";
import { openPlaceholder, placeholderTail } from "./template";

export interface Suggestion {
  label: string;
  detail?: string;
  /** The range of the text the suggestion replaces. */
  from: number;
  to: number;
  insert: string;
}

/** Suggestions for `text` with the caret at `cursor`; empty when there are none. */
export type Suggester = (text: string, cursor: number) => Suggestion[];

const MAX_SUGGESTIONS = 50;

/** The first suggester with anything to offer wins. */
export const firstOf =
  (...suggesters: (Suggester | undefined)[]): Suggester =>
  (text, cursor) => {
    for (const suggester of suggesters) {
      const found = suggester?.(text, cursor) ?? [];
      if (found.length > 0) return found;
    }
    return [];
  };

/** Names starting with the query first, then names containing it. */
function rankByQuery(names: readonly string[], query: string): string[] {
  const lower = query.toLowerCase();
  const starts = names.filter((name) => name.toLowerCase().startsWith(lower));
  const contains = names.filter((name) => !name.toLowerCase().startsWith(lower) && name.toLowerCase().includes(lower));
  return [...starts, ...contains];
}

/** Completes the whole field from `words`, best matches first; nothing once it's one of them. */
export const wordSuggester =
  (words: readonly string[]): Suggester =>
  (text, cursor) => {
    const typed = text.slice(0, cursor).trim();
    if (words.includes(text.trim())) return [];
    return rankByQuery(words, typed)
      .slice(0, MAX_SUGGESTIONS)
      .map((word) => ({ label: word, from: 0, to: text.length, insert: word }));
  };

/** Completes the variable name inside an open `{{`. */
export const placeholderSuggester =
  (names: readonly string[], details: Readonly<Record<string, string>> = {}): Suggester =>
  (text, cursor) => {
    const open = openPlaceholder(text, cursor);
    if (!open) return [];
    const to = cursor + placeholderTail(text, cursor);
    return rankByQuery(names, open.query)
      .slice(0, MAX_SUGGESTIONS)
      .map((name) => {
        const suggestion: Suggestion = { label: name, from: open.start, to, insert: `{{${name}}}` };
        const detail = details[name];
        if (detail) suggestion.detail = detail;
        return suggestion;
      });
  };

/** The rest of the word the caret is in, so a completion replaces all of it. */
const wordEnd = (text: string, cursor: number): number => cursor + (/^\S*/.exec(text.slice(cursor))?.[0].length ?? 0);

const RESPONSE_ROOTS: { insert: string; detail: string }[] = [
  { insert: "res.status", detail: "status code" },
  { insert: "res.body.", detail: "JSON body" },
  { insert: "res.headers.", detail: "header" },
  { insert: "res.responseTime", detail: "ms" },
];

const BODY = "res.body";
const HEADERS = "res.headers.";

/** `.a[0]['b c']` → `["a", 0, "b c"]`, or null if it isn't a path. */
function pathSegments(path: string): (string | number)[] | null {
  const segments: (string | number)[] = [];
  const pattern = /\.([^.[\]]+)|\[(\d+)\]|\[["']([^"']*)["']\]/y;
  let at = 0;
  while (at < path.length) {
    pattern.lastIndex = at;
    const match = pattern.exec(path);
    if (!match) return null;
    segments.push(match[2] !== undefined ? Number(match[2]) : (match[1] ?? match[3] ?? ""));
    at = pattern.lastIndex;
  }
  return segments;
}

function valueAt(root: unknown, segments: (string | number)[]): unknown {
  let value = root;
  for (const segment of segments) {
    if (value === null || typeof value !== "object") return undefined;
    value = (value as Record<string | number, unknown>)[segment];
  }
  return value;
}

function describe(value: unknown): string {
  if (Array.isArray(value)) return "array";
  if (value === null) return "null";
  return typeof value;
}

const IDENTIFIER = /^[A-Za-z_$][\w$]*$/;

function parseBody(response: HttpResponse | null): unknown {
  if (!response?.body) return undefined;
  try {
    return JSON.parse(response.body) as unknown;
  } catch {
    return undefined;
  }
}

/** The keys under the path typed so far, read from the last response's JSON body. */
function bodySuggestions(typed: string, body: unknown, to: number): Suggestion[] {
  const rest = typed.slice(BODY.length);
  const lastDot = rest.lastIndexOf(".");
  if (lastDot < 0 || body === undefined) return [];
  const parentPath = rest.slice(0, lastDot);
  const partial = rest.slice(lastDot + 1);
  if (/[[\]]/.test(partial)) return [];
  const segments = pathSegments(parentPath);
  const parent = segments && valueAt(body, segments);
  const base = `${BODY}${parentPath}`;
  if (Array.isArray(parent)) {
    return partial ? [] : [{ label: "[0]", detail: `${parent.length} items`, from: 0, to, insert: `${base}[0]` }];
  }
  if (parent === null || typeof parent !== "object") return [];
  return rankByQuery(Object.keys(parent), partial)
    .slice(0, MAX_SUGGESTIONS)
    .map((key) => ({
      label: key,
      detail: describe((parent as Record<string, unknown>)[key]),
      from: 0,
      to,
      insert: IDENTIFIER.test(key) ? `${base}.${key}` : `${base}['${key}']`,
    }));
}

function headerSuggestions(typed: string, response: HttpResponse | null, to: number): Suggestion[] {
  const names = [...new Set((response?.headers ?? []).map(([name]) => name.toLowerCase()))];
  return rankByQuery(names, typed.slice(HEADERS.length))
    .slice(0, MAX_SUGGESTIONS)
    .map((name) => ({ label: name, detail: "header", from: 0, to, insert: `${HEADERS}${name}` }));
}

/**
 * Completes `res.status`, `res.body.…` and `res.headers.…` expressions, with
 * body keys and header names taken from `response` when there is one.
 */
export function expressionSuggester(response: HttpResponse | null): Suggester {
  let body: unknown;
  let parsed = false;
  const lazyBody = () => {
    if (!parsed) {
      body = parseBody(response);
      parsed = true;
    }
    return body;
  };
  return (text, cursor) => {
    const typed = text.slice(0, cursor);
    if (/\s/.test(typed)) return [];
    const to = wordEnd(text, cursor);
    if (typed.startsWith(HEADERS)) return headerSuggestions(typed, response, to);
    if (typed.startsWith(`${BODY}.`) || typed.startsWith(`${BODY}[`)) return bodySuggestions(typed, lazyBody(), to);
    return RESPONSE_ROOTS.filter((root) => root.insert.startsWith(typed) && root.insert !== typed).map((root) => ({
      label: root.insert,
      detail: root.detail,
      from: 0,
      to,
      insert: root.insert,
    }));
  };
}

/** Operators that take no value: `isDefined`, `isEmpty`, and so on. */
const UNARY = new Set<string>(ASSERTION_OPERATORS.filter((op) => op.startsWith("is")));

/** Completes the operator at the start of an assertion condition such as `eq 200`. */
export const operatorSuggester: Suggester = (text, cursor) => {
  const typed = text.slice(0, cursor);
  if (/\s/.test(typed) || (ASSERTION_OPERATORS as readonly string[]).includes(typed)) return [];
  const to = wordEnd(text, cursor);
  const hasValue = text.slice(to).trim() !== "";
  return rankByQuery(ASSERTION_OPERATORS, typed)
    .filter((op) => op !== typed)
    .map((op) => ({
      label: op,
      detail: UNARY.has(op) ? "" : "value",
      from: 0,
      to,
      insert: UNARY.has(op) || hasValue ? op : `${op} `,
    }));
};
